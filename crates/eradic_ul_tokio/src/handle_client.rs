use core::net::SocketAddr;
use std::io::Cursor;

use eradic_common::DeserializedPdu;
use eradic_common::ul::associate::abort::{AbortReason, serialize_abort_pdu};
use eradic_common::ul::associate::{PduDeserializationError, deserialized_pdu_from_reader, serialize_associate_pdu};
use eradic_common::ul::pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header};
use eradic_common::ul::service::ProviderAbortIndication;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::oneshot;
use tokio::{
    net::TcpStream,
    sync::mpsc,
    task::JoinSet,
};

use tracing::{info, instrument, warn};

use eradic_common::ul::connection::{UpperLayerConnection, UpperLayerConnectionState};
use eradic_common::ul::event::{Command, Event, Indication, event_from_deserialized_pdu};

use crate::artim::artim_task;

use crate::{HandleClientError};

#[instrument(skip_all)]
pub async fn handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    connection: UpperLayerConnection,
    scu_handler: F,
    initial_events: Vec<Event>
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

    let (reader, writer) = tcp.into_split();

    let (command_tx, command_rx) = mpsc::channel::<(Command, UpperLayerConnectionState)>(32);
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);
    let (user_tx, user_rx) = mpsc::channel::<Indication>(32);

    let mut set: JoinSet<Result<(), HandleClientError>> = JoinSet::new();

    set.spawn(user_connection_task(user_rx, event_tx.clone(), scu_handler));

    set.spawn(handle_event_task(
        event_rx,
        command_tx.clone(),
        connection
    ));

    set.spawn(handle_command_task(writer, command_rx, user_tx, event_tx.clone()));

    set.spawn(pdu_read_task(reader, event_tx.clone()));

    for event in initial_events {
        event_tx.send(event).await;
    }

    drop(event_tx);

    let mut client_result: Result<(), HandleClientError> = Ok(());

    if let Some(result) = set.join_next().await {
        match result {
            Ok(Err(e)) => {
                warn!("Task exited unexpectedly: {:?}", e);
                client_result = Err(e);
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

    client_result
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
    command_tx: mpsc::Sender<(Command, UpperLayerConnectionState)>,
    mut conn: UpperLayerConnection,
) -> Result<(), HandleClientError> {
    while let Some(event) = event_rx.recv().await {
        info!("Event received: {:?}", event);

        match conn.handle_event(event) {
            Ok(commands) => {
                for cmd in commands {
                    command_tx.send((cmd, conn.state)).await;
                }
            }
            Err(e) => {
                command_tx.send((
                    Command::ProviderAbortIndication(
                        ProviderAbortIndication::new(AbortReason::NoReason)
                    ),
                    conn.state
                )).await;

                return Err(e.into());
            }
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<(Command, UpperLayerConnectionState)>,
    user_connection: mpsc::Sender<Indication>,
    event_tx: mpsc::Sender<Event>,
) -> Result<(), HandleClientError>
where
    W: AsyncWrite + Unpin,
{
    let mut artim_cancel: Option<oneshot::Sender<()>> = None;

    while let Some((command, state)) = rx.recv().await {
        match command {
            // Acceptor and Requestor commands
            Command::AbortIndication(indication) => {
                user_connection
                    .send(Indication::AbortIndication(indication))
                    .await;

                writer.shutdown().await;
                return Ok(());
            }

            Command::ProviderAbortIndication(indication) => {
                user_connection
                    .send(Indication::ProviderAbortIndication(indication))
                    .await;
            }

            Command::StartArtimTimer => {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                tokio::spawn(artim_task(cancel_rx, event_tx.clone()));
                artim_cancel = Some(cancel_tx);
            }

            Command::StopArtimTimer => {
                artim_cancel = None;
            }

            Command::CloseConnection => {
                writer.shutdown().await;
                return Ok(());
            }

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

            _ => todo!(),
        };

        if state == UpperLayerConnectionState::Idle {
            return Ok(())
        }
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
