use core::net::SocketAddr;
use std::io::Cursor;
use std::net::IpAddr;
use thiserror::Error;

use tokio::io::AsyncWrite;
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
    task::JoinSet,
};

use tracing::{info, instrument};
use tracing_log::log::{error, warn};

use eradic_common::ul::associate::{
    AssociateRqAcPdu, PduDeserializationError, deserialized_pdu_from_reader,
    serialize_associate_pdu,
};
use eradic_common::ul::connection::{UpperLayerAcceptorConnection, handle_server_event};
use eradic_common::ul::event::{Command, Event, Indication, event_from_deserialized_pdu};
use eradic_common::ul::pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header};
use eradic_common::ul::service::AssociateRequestResponse;

#[derive(Debug, Error)]
pub enum IoError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
}

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Error)]
pub enum TaskError {
    #[error(transparent)]
    ReadError(#[from] IoError),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
}

#[instrument(skip(tcp, scu_handler) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub async fn handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    scu_handler: F,
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

    let mut set: JoinSet<Result<(), TaskError>> = JoinSet::new();

    set.spawn(user_connection_task(user_rx, event_tx.clone(), scu_handler));

    set.spawn(handle_event_task(
        event_rx,
        command_tx.clone(),
        writer.local_addr()?.ip(),
        writer.local_addr()?.port(),
        socket_addr.ip(),
        socket_addr.port(),
    ));

    set.spawn(handle_command_task(writer, command_rx, user_tx));

    // TODO: Move to other functions, too nested
    set.spawn(pdu_read_task(reader, event_tx));

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

#[instrument(skip(reader, event_tx))]
async fn pdu_read_task<R>(mut reader: R, event_tx: mpsc::Sender<Event>) -> Result<(), TaskError>
where
    R: AsyncRead + Unpin,
{
    loop {
        let (buf, pdu_type) = match read_full_pdu(&mut reader).await {
            Ok(val) => val,
            Err(e) => {
                event_tx
                    .send(Event::TransportConnectionClosedIndication)
                    .await;
                return Err(TaskError::from(e));
            }
        };
        info!("PDU received: {:?}", pdu_type);
        event_tx
            .send(event_from_deserialized_pdu(deserialized_pdu_from_reader(
                &mut Cursor::new(buf),
                pdu_type,
            )?))
            .await;
    }
}

#[instrument]
async fn handle_event_task(
    mut event_rx: mpsc::Receiver<Event>,
    command_tx: mpsc::Sender<Command>,
    called: IpAddr,
    called_port: u16,
    calling: IpAddr,
    calling_port: u16,
) -> Result<(), TaskError> {
    let mut conn =
        UpperLayerAcceptorConnection::new_server(called, called_port, calling, calling_port);

    while let Some(event) = event_rx.recv().await {
        info!("Event received");

        let (command, new_state) = handle_server_event(conn, event).unwrap();

        conn = new_state;

        if let Some(cmd) = command {
            command_tx.send(cmd).await;
        }
    }

    info!("Exiting");
    Ok(())
}

#[instrument(skip(writer))]
async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    user_connection: mpsc::Sender<Indication>,
) -> Result<(), TaskError>
where
    W: AsyncWrite + Unpin,
{
    while let Some(command) = rx.recv().await {
        match command {
            Command::AssociateAcceptPdu(resp) => {
                handle_Associate_response(AssociateRequestResponse::Accepted(resp), &mut writer)
                    .await;
            }

            Command::AssociateIndication(indication) => {
                user_connection
                    .send(Indication::AssociateIndication(indication))
                    .await;
            }

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

#[instrument(skip(scu_handler))]
async fn user_connection_task<F, Fut>(
    mut rx: mpsc::Receiver<Indication>,
    event_tx: mpsc::Sender<Event>,
    scu_handler: F,
) -> Result<(), TaskError>
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

#[instrument(skip(reader))]
async fn read_full_pdu<R>(reader: &mut R) -> Result<(Vec<u8>, PduType), IoError>
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

#[instrument(skip(tcp))]
async fn handle_Associate_response<W>(response: AssociateRequestResponse, tcp: &mut W)
where
    W: AsyncWrite + Unpin,
{
    match response {
        AssociateRequestResponse::Accepted(inner) => {
            info!("Sending A-Associate-AC PDU");

            let pdu = AssociateRqAcPdu::try_from(inner).unwrap();
            tcp.write_all(serialize_associate_pdu(&pdu).as_slice())
                .await;
        }
        AssociateRequestResponse::Rejected(inner) => {
            info!("Sending A-Associate-RJ PDU: {:?}", inner);
            todo!();
        }
    }
}
