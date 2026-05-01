use core::net::SocketAddr;
use std::io::{Cursor, Read};
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use thiserror::Error;

use tokio::io::AsyncWrite;
use tokio::time::{self, Sleep};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{mpsc},
};
use tokio_util::sync::CancellationToken;

use tracing::{Instrument, Level, info, span};
use tracing_log::log::{error, warn};
use tracing_subscriber::{FmtSubscriber, fmt};

use eradic_common::associate::{
    AssociateRqAcPdu, PduDeserializationError, deserialized_pdu_from_reader, event_from_deserialized_pdu, serialize_associate_pdu
};
use eradic_common::connection::{UpperLayerAcceptorConnection, handle_server_event};
use eradic_common::event::{Command, Event, Indication};
use eradic_common::pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header};
use eradic_common::service::AssociateRequestResponse;

#[derive(Debug, Error)]
pub enum ServiceUserError {
    #[error("Service User / Application Entity was not found")]
    ServiceUserNotFound,
}

#[derive(Debug, Error)]
pub enum ReadError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError)
}

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub async fn handle_client<F, Fut>(tcp: TcpStream, socket_addr: SocketAddr, scu_handler: F) -> Result<(), HandleClientError>
where
    F: Fn(Indication) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<Event>> + Send + 'static,
{
    let span = span!(Level::INFO, "connection",
        ip = %socket_addr.ip(),
        port = socket_addr.port()
    );

    let _guard = span.enter();

    info!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let (mut reader, writer) = tcp.into_split();

    let called_address = writer.local_addr()?.ip();
    let called_port = writer.local_addr()?.port();

    let (command_tx, command_rx) = mpsc::channel::<Command>(32);
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);
    let (user_tx, mut user_rx) = mpsc::channel::<Indication>(32);

    let cancel = CancellationToken::new();
    let child = cancel.child_token();
    let child_2 = cancel.child_token();

    let event_tx_user = event_tx.clone();

    let user_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU connection task",
    );

    let user_task = tokio::spawn(
        async move {
            while let Some(indication) = user_rx.recv().await {
                info!(
                    "Received indication: {}",
                    <&str>::from(&indication)
                );

                if let Some(event) = scu_handler(indication).await {
                    event_tx_user.send(event).await;
                }
            }

            info!("All writers out of scope, exiting task");
        }
        .instrument(user_span)
    );

    let event_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU event task",
    );
    let event_task = tokio::spawn(
        handle_event_task(
            event_rx,
            command_tx.clone(),
            writer.local_addr()?.ip(),
            writer.local_addr()?.port(),
            socket_addr.ip(),
            socket_addr.port(),
            child,
        )
        .instrument(event_span),
    );

    let command_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU command task",
    );
    let command_task = tokio::spawn(
        handle_command_task(writer, command_rx, user_tx, cancel)
            .instrument(command_span),
    );


    let read_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU PDU read task",
    );

    // TODO: Move to other functions, too nested
    let read_task: tokio::task::JoinHandle<Result<(), ReadError>> = tokio::spawn(async move {
        tokio::select! {
            _ = child_2.cancelled() => {
                return Ok(())
            }
            result = async {
                loop {
                    let (buf, pdu_type) = read_full_pdu(&mut reader).await?;
                    info!("PDU received: {:?}", pdu_type);

                    event_tx.send(
                        event_from_deserialized_pdu(
                            deserialized_pdu_from_reader(&mut Cursor::new(buf), pdu_type)?
                        )
                    ).await;
                }
            } => {
                warn!("Task exited unexpectedly: {:?}", result);

                event_tx.send(
                    Event::TransportConnectionClosedIndication
                ).await;

                result
            }
        }
    }.instrument(read_span));

    tokio::join!(command_task, event_task, user_task, read_task);

    info!(
        "Closing connection: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    Ok(())
}

async fn handle_event_task(
    mut event_rx: mpsc::Receiver<Event>,
    command_tx: mpsc::Sender<Command>,
    called: IpAddr,
    called_port: u16,
    calling: IpAddr,
    calling_port: u16,
    cancel: CancellationToken,
) {
    let mut conn = UpperLayerAcceptorConnection::new_server(called, called_port, calling, calling_port);

    tokio::select! {
        _ = cancel.cancelled() => {
            info!("Cancelled");
        }
        _ = async {
            while let Some(event) = event_rx.recv().await {
                info!("Event received");

                let (command, new_state) = handle_server_event(
                    conn,
                    event,
                )
                .unwrap();

                conn = new_state;

                if let Some(cmd) = command {
                    command_tx.send(cmd).await;
                }
            }
        } => {}
    }
}

async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    user_connection: mpsc::Sender<Indication>,
    cancel: CancellationToken,
)
where
    W: AsyncWrite + Unpin,
{
    tokio::select! {
        _ = cancel.cancelled() => {
            info!("Cancelled");
        }
        _ = async {
            while let Some(command) = rx.recv().await {
                handle_command(&mut writer, command, user_connection.clone(), cancel.clone()).await;
            }
        } => {}
    }
}

async fn handle_command<W>(
    writer: &mut W,
    command: Command,
    user_connection: mpsc::Sender<Indication>,
    cancel: CancellationToken,
)
where
    W: AsyncWrite + Unpin,
{
    info!("Command received");
    match command {
        Command::AssociateAcceptPdu(resp) => {
            handle_Associate_response(AssociateRequestResponse::Accepted(resp), writer).await;
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

            cancel.cancel();
        }

        Command::ProviderAbortIndication(indication) => {
            user_connection
                .send(Indication::ProviderAbortIndication(indication))
                .await;

            cancel.cancel();
        }

        _ => todo!(),
    };
}

async fn read_full_pdu<R>(reader: &mut R) -> Result<(Vec<u8>, PduType), ReadError>
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

async fn handle_Associate_response<W>(
    response: AssociateRequestResponse,
    tcp: &mut W,
)
where
    W: AsyncWrite + Unpin,
{
    match response {
        AssociateRequestResponse::Accepted(inner) => {
            info!("Sending A-Associate-AC PDU");

            let pdu = AssociateRqAcPdu::from_response(&inner).unwrap();
            tcp.write_all(serialize_associate_pdu(&pdu).as_slice())
                .await;
        }
        AssociateRequestResponse::Rejected(inner) => {
            info!("Sending A-Associate-RJ PDU: {:?}", inner);
            todo!();
        }
    }
}
