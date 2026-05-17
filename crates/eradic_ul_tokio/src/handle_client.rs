use core::net::SocketAddr;
use std::io::Cursor;

use eradic_common::DeserializedPdu;
use eradic_common::ul::associate::abort::serialize_abort_pdu;
use eradic_common::ul::associate::{PduDeserializationError, deserialized_pdu_from_reader, serialize_associate_pdu};
use eradic_common::ul::pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::{
    net::TcpStream,
    sync::mpsc,
    task::JoinSet,
};

use tracing::{info, instrument};
use tracing_log::log::{warn};

use eradic_common::ul::connection::{UpperLayerConnection};
use eradic_common::ul::event::{Command, Event, Indication, event_from_deserialized_pdu};

use crate::{HandleClientError};

#[instrument(skip_all)]
pub async fn handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    mut connection: UpperLayerConnection,
    scu_handler: F,
    initial_commands: Vec<Command>
) -> Result<(), HandleClientError>
where
    F: Fn(Indication) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<Event>> + Send + 'static,
{
    info!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let (mut reader, writer) = tcp.into_split();

    let (command_tx, command_rx) = mpsc::channel::<Command>(32);
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);
    let (user_tx, mut user_rx) = mpsc::channel::<Indication>(32);

    let mut set: JoinSet<Result<(), HandleClientError>> = JoinSet::new();

    set.spawn(user_connection_task(user_rx, event_tx.clone(), scu_handler));

    set.spawn(handle_event_task(
        event_rx,
        command_tx.clone(),
        connection
    ));

    set.spawn(handle_command_task(writer, command_rx, user_tx));

    set.spawn(pdu_read_task(reader, event_tx));

    for cmd in initial_commands {
        println!("{}", cmd);
        command_tx.send(cmd).await;
    }

    if let Some(result) = set.join_next().await {
        match result {
            Ok(Err(e)) => {
                warn!("Task exited unexpectedly: {:?}", e);
            }
            _ => {}
        }

        set.join_all().await;
    }

    info!(
        "Closing connection: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    Ok(())
}

#[instrument(skip_all)]
async fn user_connection_task<F, Fut>(
    mut rx: mpsc::Receiver<Indication>,
    event_tx: mpsc::Sender<Event>,
    scu_handler: F,
) -> Result<(), HandleClientError>
where
    F: Fn(Indication) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<Event>> + Send + 'static,
{
    while let Some(indication) = rx.recv().await {
        info!("Received indication: {}", <&str>::from(&indication));

        if let Some(event) = scu_handler(indication).await {
            event_tx.send(event).await;
        }
    }

    info!("All writers out of scope, exiting task");
    Ok(())
}

#[instrument(skip_all)]
async fn pdu_read_task<R>(mut reader: R, event_tx: mpsc::Sender<Event>) -> Result<(), HandleClientError>
where
    R: AsyncRead + Unpin,
{
    loop {
        let (buf, pdu_type) = match read_full_pdu(&mut reader).await {
            Ok(val) => val,
            Err(ReadPduError::DeserializeError(e)) => {
                event_tx
                    .send(Event::UnrecognizedPdu)
                    .await;

                return Err(HandleClientError::PduDeserializationError(e));
            }
            Err(ReadPduError::Io(e)) => {
                event_tx
                    .send(Event::TransportConnectionClosedIndication)
                    .await;

                return Err(HandleClientError::Io(e));
            }
        };
        info!("PDU received: {:?}", pdu_type);

        match deserialized_pdu_from_reader(&mut Cursor::new(buf), pdu_type) {
            Ok(pdu) => {
                event_tx.send(event_from_deserialized_pdu(pdu)).await;
            }
            Err(PduDeserializationError::UnrecognizedItemType(item_type)) => {
                event_tx
                    .send(Event::UnrecognizedPduParameter)
                    .await;

                return Err(HandleClientError::PduDeserializationError(
                    PduDeserializationError::UnrecognizedItemType(item_type)
                ));
            }

            Err(PduDeserializationError::UnexpectedItemType(item_type)) => {
                event_tx
                    .send(Event::UnexpectedPduParameter)
                    .await;

                return Err(HandleClientError::PduDeserializationError(
                    PduDeserializationError::UnexpectedItemType(item_type)
                ));
            }

            Err(e) => {
                event_tx
                    .send(Event::InvalidPduParameter)
                    .await;

                return Err(HandleClientError::PduDeserializationError(e));
            }
        }
    }
}

#[instrument(skip_all)]
async fn handle_event_task(
    mut event_rx: mpsc::Receiver<Event>,
    command_tx: mpsc::Sender<Command>,
    mut conn: UpperLayerConnection,
) -> Result<(), HandleClientError> {
    while let Some(event) = event_rx.recv().await {
        info!("Event received");

        let commands = conn.handle_event(event).unwrap();

        for cmd in commands {
            command_tx.send(cmd).await;
        }
    }

    info!("Exiting");
    Ok(())
}

#[instrument(skip_all)]
async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    user_connection: mpsc::Sender<Indication>,
) -> Result<(), HandleClientError>
where
    W: AsyncWrite + Unpin,
{
    while let Some(command) = rx.recv().await {
        match command {
            // Acceptor commands
            Command::AssociateAcceptPdu(pdu) => {
                stream_write_pdu(DeserializedPdu::AssociateAccept(pdu), &mut writer).await;
            }

            Command::AssociateIndication(indication) => {
                user_connection
                    .send(Indication::AssociateIndication(indication))
                    .await;
            }

            // Requestor commands
            Command::AssociateRequestPdu(pdu) => {
                stream_write_pdu(DeserializedPdu::AssociateRequest(pdu), &mut writer).await;
            }

            // Acceptor and Requestor commands
            Command::AbortIndication(indication) => {
                user_connection
                    .send(Indication::AbortIndication(indication))
                    .await;

                return Ok(());
            }

            Command::ProviderAbortIndication(indication) => {
                user_connection
                    .send(Indication::ProviderAbortIndication(indication))
                    .await;

                return Ok(());
            }

            _ => todo!(),
        };
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum ReadPduError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DeserializeError(#[from] PduDeserializationError)
}

#[instrument(skip_all)]
async fn read_full_pdu<R>(reader: &mut R) -> Result<(Vec<u8>, PduType), ReadPduError>
where
    R: AsyncRead + Unpin,
{
    let mut header_buf = [0u8; 6];
    reader.read_exact(&mut header_buf).await?;

    let mut cursor = Cursor::new(header_buf);
    let header = read_pdu_header(&mut cursor)?;
    info!("{:?}", header);

    let mut buffer = vec![0u8; PDU_HEADER_LENGTH + header.length as usize];
    buffer[..6].copy_from_slice(&header_buf);

    reader.read_exact(&mut buffer[6..]).await?;

    Ok((buffer, header.pdu_type))
}

#[instrument(skip_all)]
async fn stream_write_pdu<W>(response: DeserializedPdu, tcp: &mut W)
where
    W: AsyncWrite + Unpin,
{
    info!("Sending PDU: {}", <&str>::from(&response));

    match response {
        DeserializedPdu::AssociateRequest(pdu) |
        DeserializedPdu::AssociateAccept(pdu) => {
            tcp.write_all(serialize_associate_pdu(&pdu).as_slice()).await;
        }

        DeserializedPdu::Abort(pdu) => {
            tcp.write_all(serialize_abort_pdu(&pdu).as_slice()).await;
        }

        _ => {
            todo!()
        }
    }
}
