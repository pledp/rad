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
use tracing_log::log::error;
use tracing_subscriber::{FmtSubscriber, fmt};

use eradic_common::associate::{
    AssociateRqAcPdu, deserialize_associate_pdu, deserialized_pdu_from_reader, event_from_deserialized_pdu, serialize_associate_pdu
};
use eradic_common::connection::{UpperLayerAcceptorConnection, handle_server_event};
use eradic_common::event::{Command, Event, Indication};
use eradic_common::pdu::{DeserializedPdu, PDU_HEADER_LENGTH, PduType, read_pdu_header};

use eradic_common::service::AssociateRequestResponse;

use eradic_adaptor::{UpperLayerServiceUser, UpperLayerServiceUserAsync};

use crate::service_user::LocalUpperLayerServiceUser;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_log::LogTracer::init()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .fmt_fields(fmt::format::DefaultFields::new())
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("System initialized");

    let ul_scu = Arc::new(LocalUpperLayerServiceUser::new());

    let server = TcpListener::bind("127.0.0.1:104").await?;
    info!("Listening for connections...");

    let client_count = Arc::new(AtomicI64::new(0));

    loop {
        let (tcp, socket_addr) = server.accept().await?;
        let client_count_clone = Arc::clone(&client_count);
        let ul_scu_clone = Arc::clone(&ul_scu);

        tokio::spawn(async move {
            client_count_clone.fetch_add(1, Ordering::AcqRel);

            let result = handle_client(tcp, socket_addr, ul_scu_clone).await;

            client_count_clone.fetch_sub(1, Ordering::AcqRel);

            result
        });
    }

    Ok(())
}

async fn handle_client<U>(tcp: TcpStream, socket_addr: SocketAddr, ul_scu: Arc<U>) -> Result<()>
where
    U: UpperLayerServiceUserAsync + Sync + Send + 'static
{
    let span = span!(Level::INFO, "connection",
        ip = %socket_addr.ip(),
        port = socket_addr.port()
    );

    span.record("Task", &"User task");

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
    let (user_tx, mut user_rx) = mpsc::channel::<Indication>(32);

    let cancel = CancellationToken::new();
    let child = cancel.child_token();

    let event_tx_user = event_tx.clone();

    let user_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU connection task",
    );
    let user_task = tokio::spawn(
        async move {
            loop {
                let indication = user_rx.recv().await.unwrap();
                info!(
                    "Received indication: {}",
                    <&str>::from(&indication)
                );

                match indication {
                    Indication::AssociateIndication(indication) => {
                        info!("Creating SCU connection");

                        let event = ul_scu.handle_associate_request(indication).await;

                        event_tx_user.send(event).await;
                    },
                    Indication::AbortIndication(indication) => {
                        return;
                    }
                    _ => todo!(),
                }
            }
        }
        .instrument(user_span)
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

    let event_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU event task",
    );
    let event_task = tokio::spawn(
        handle_event_task(
            event_rx,
            command_tx.clone(),
            called,
            socket_addr.ip(),
            child,
        )
        .instrument(event_span),
    );

    let read_span = span!(
        parent: span.clone(),
        Level::INFO,
        "UL SCU PDU read task",
    );
    // TODO: Move to other functions, too nested
    let read_task: tokio::task::JoinHandle<Result<String>> = tokio::spawn(
        async move {
            loop {
                match read_full_pdu(&mut reader).await {
                    Ok((buf, pdu_type)) => {
                        event_tx.send(
                            event_from_deserialized_pdu(
                                deserialized_pdu_from_reader(&mut Cursor::new(buf), pdu_type)?
                            )
                        ).await;
                    }
                    Err(_e) => {
                        event_tx.send(
                            Event::TransportConnectionClosedIndication
                        );
                        error!("READ ERROR");
                        return Err("test".into())
                    }
                }
            }
        }
        .instrument(read_span),
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
    let mut conn = UpperLayerAcceptorConnection::new_server(called, calling);

    tokio::select! {
        _ = cancel.cancelled() => {
            info!("Cancelled");
        }
        _ = async {
            while let Some(event) = event_rx.recv().await {
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
            }
        } => {}
    }

    Ok(())
}

async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    user_connection: mpsc::Sender<Indication>,
    cancel: CancellationToken,
) -> Result<()>
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

    Ok(())
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
            tcp.write_all(serialize_associate_pdu(&pdu).as_slice())
                .await?;
            Ok(())
        }
        AssociateRequestResponse::Rejected(inner) => {
            info!("Sending A-Associate-RJ PDU: {:?}", inner);
            todo!();
        }
    }
}
