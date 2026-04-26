mod service_user;

use core::net::SocketAddr;
use std::io::Cursor;
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};
use std::thread::sleep;

use tokio::io::AsyncWrite;
use tokio::time::{self, Sleep};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc},
};
use tokio_util::sync::CancellationToken;

use tracing::{Instrument, Level, info, span};
use tracing_subscriber::FmtSubscriber;

use eradic_common::associate::{
    AssociateRqAcPdu, deserialize_Associate_pdu, deserialized_pdu_from_reader, event_from_deserialized_pdu, serialize_Associate_pdu
};
use eradic_common::connection::{UpperLayerAcceptorConnection, handle_server_event};
use eradic_common::event::{Command, Event};
use eradic_common::pdu::{DeserializedPdu, PDU_HEADER_LENGTH, PduType, read_pdu_header};

use eradic_common::service::AssociateRequestResponse;

use eradic_adaptor::UpperLayerServiceUserConnection;

use crate::service_user::ServiceUsers;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_log::LogTracer::init()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("System initialized");

    let server = TcpListener::bind("127.0.0.1:104").await?;
    info!("Listening for connections...");

    let client_count = Arc::new(AtomicI64::new(0));

    loop {
        let (tcp, socket_addr) = server.accept().await?;
        let client_count_clone = Arc::clone(&client_count);

        tokio::spawn(async move {
            client_count_clone.fetch_add(1, Ordering::AcqRel);

            let result = handle_client(tcp, socket_addr).await;

            client_count_clone.fetch_sub(1, Ordering::AcqRel);

            result
        });
    }

    Ok(())
}

async fn handle_client(tcp: TcpStream, socket_addr: SocketAddr) -> Result<()> {
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

    let called = writer.local_addr()?.ip();

    let (command_tx, command_rx) = mpsc::channel::<Command>(32);
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);
    let (user_tx, mut user_rx) = mpsc::channel::<Command>(32);

    let cancel = CancellationToken::new();
    let child = cancel.child_token();
    let child_2 = cancel.child_token();

    let event_tx_user = event_tx.clone();
    let user_task = tokio::spawn(
        async move {
            let mut conn: Option<Box<dyn UpperLayerServiceUserConnection>> = None;

            loop {
                info!("Starting user task");
                let command = user_rx.recv().await.unwrap();
                info!("User task: Command received");

                match command {
                    Command::AssociateIndication(indication) => {
                        info!("User task: Received Command::AssociateIndication");

                        info!("User task: Creating SCU connection");
                        conn = Some(
                            ServiceUsers::create_scu_connection(&indication.called_ae).unwrap(),
                        );

                        let event = if let Some(c) = conn.as_mut() {
                            c.handle_associate_request(indication)
                        } else {
                            todo!()
                        };

                        sleep(time::Duration::from_secs(500));

                        event_tx_user.send(event).await;
                    }
                    _ => todo!(),
                }
            }
        }
        .instrument(tracing::Span::current()),
    );

    let command_task = tokio::spawn(
        handle_command_task(writer, command_rx, user_tx, child)
            .instrument(tracing::Span::current()),
    );

    let event_task = tokio::spawn(
        handle_event_task(
            event_rx,
            command_tx.clone(),
            called,
            socket_addr.ip(),
            child_2,
        )
        .instrument(tracing::Span::current()),
    );

    // TODO: Move to other functions, too nested
    let read_task: tokio::task::JoinHandle<Result<String>> = tokio::spawn(
        async move {
            info!("Read task started");

            loop {
                match read_full_pdu(&mut reader).await {
                    Ok((buf, pdu_type)) => {
                        event_tx.send(
                            event_from_deserialized_pdu(
                                deserialized_pdu_from_reader(&mut Cursor::new(buf), pdu_type)?
                            )
                        ).await;
                    }
                    Err(_e) => return Err("test".into()),
                }
            }
        }
        .instrument(tracing::Span::current()),
    );

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
    calling: IpAddr,
    cancel: CancellationToken,
) -> Result<()> {
    info!("Event task started");
    let mut conn = UpperLayerAcceptorConnection::new_server(called, calling);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("cancelled");
                break;
            }
            _ = async {
                let event = event_rx.recv().await.unwrap();
                info!("Event received");

                let (command, new_state) = handle_server_event(
                    &conn,
                    event,
                )
                .unwrap();

                conn = new_state;

                if let Some(cmd) = command {
                    command_tx.send(cmd).await;
                }

            } => {}
        }
    }

    Ok(())
}

async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    user_connection: mpsc::Sender<Command>,
    cancel: CancellationToken,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    info!("Command task started");

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("cancelled");
                break;
            }
            _ = async {
                let command = rx.recv().await.unwrap();

                handle_command(&mut writer, command, user_connection.clone()).await;
            } => {}
        }
    }

    Ok(())
}

async fn handle_command<W>(writer: &mut W, command: Command, user_connection: mpsc::Sender<Command>)
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
                .send(Command::AssociateIndication(indication))
                .await;
        }

        _ => todo!(),
    };
}

async fn read_full_pdu<R>(reader: &mut R) -> Result<(Vec<u8>, PduType)>
where
    R: AsyncRead + Unpin,
{
    let mut header_buf = [0u8; 6];
    reader.read_exact(&mut header_buf).await?;

    let mut cursor = Cursor::new(header_buf);
    let header = read_pdu_header(&mut cursor)?;

    let mut buffer = vec![0u8; PDU_HEADER_LENGTH + header.length as usize];
    buffer[..6].copy_from_slice(&header_buf);

    reader.read_exact(&mut buffer[6..]).await?;

    Ok((buffer, header.pdu_type))
}

async fn handle_Associate_response<W>(
    response: AssociateRequestResponse,
    tcp: &mut W,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    match response {
        AssociateRequestResponse::Accepted(inner) => {
            info!("Sending A-Associate-AC PDU");

            let pdu = AssociateRqAcPdu::from_response(&inner)?;
            tcp.write_all(serialize_Associate_pdu(&pdu)?.as_slice())
                .await?;
            Ok(())
        }
        AssociateRequestResponse::Rejected(inner) => {
            info!("Sending A-Associate-RJ PDU: {:?}", inner);
            todo!();
        }
    }
}
