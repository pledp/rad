mod service_user;

use core::net::SocketAddr;
use std::io::{BufReader, Cursor};
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use tokio_util::sync::CancellationToken;
use tokio::io::AsyncWrite;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, AsyncRead},
    net::{TcpListener, TcpStream},
    sync::{Mutex, mpsc},
};

use tracing::{Level, info};
use tracing_subscriber::FmtSubscriber;

use eradic_common::associate::{
    self, AssociateRqAcPdu, deserialize_association_pdu, serialize_association_pdu
};
use eradic_common::connection::{UpperLayerAcceptorConnection, UpperLayerRequestorConnection, handle_server_event};
use eradic_common::event::{Command, Event};
use eradic_common::pdu::{DeserializedPdu, PDU_HEADER_LENGTH, PduHeader, PduType, read_pdu_header};

use eradic_common::service::AssociateRequestResponse;

use eradic_adaptor::{
    UpperLayerServiceUser, UpperLayerServiceUserAsync
};

use crate::service_user::{ServiceUser, UpperLayerServiceProviderConnection};

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

    let service_user = Arc::new(Mutex::new(ServiceUser::new()));

    let client_count = Arc::new(AtomicI64::new(0));

    loop {
        let (mut tcp, mut socket_addr) = server.accept().await?;
        let service_user_clone = Arc::clone(&service_user);
        let client_count_clone = Arc::clone(&client_count);

        tokio::spawn(async move {
            client_count_clone.fetch_add(1, Ordering::AcqRel);

            let result = handle_client(tcp, socket_addr, service_user_clone).await;

            client_count_clone.fetch_sub(1, Ordering::AcqRel);

            result
        });
    }

    Ok(())
}

enum error {
    some,
}

async fn handle_client<U: UpperLayerServiceUser + Send + 'static>(
    mut tcp: TcpStream,
    mut socket_addr: SocketAddr,
    service_user: Arc<Mutex<U>>,
) -> Result<()> {
    info!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let (mut reader, mut writer) = tcp.into_split();

    let called = writer.local_addr()?.ip();

    let (command_tx, mut command_rx) = mpsc::channel::<Command>(32);
    let (event_tx, mut event_rx) = mpsc::channel::<Event>(32);

    let cancel = CancellationToken::new();
    let child = cancel.child_token();
    let child_2 = cancel.child_token();

    let connection = UpperLayerServiceProviderConnection::new(service_user);
    let mut command_task = tokio::spawn(
        handle_command_task(writer, command_rx, event_tx.clone(), connection, child)
    );

    let mut event_task = tokio::spawn(
        handle_event_task(event_rx, command_tx.clone(), called, socket_addr.ip(), child_2)
    );

    // TODO: Move to other functions, too nested
    let read_task: tokio::task::JoinHandle<Result<String>> = tokio::spawn(async move {
        loop {
            match read_full_pdu(&mut reader).await {
                Ok((buf, pdu_type)) => {
                    let deserialized_pdu = match pdu_type {
                        PduType::AssociateRequest => {
                            Some(DeserializedPdu::AssociationRequest(
                                deserialize_association_pdu(&mut Cursor::new(buf)).unwrap()
                            ))
                        },
                        _ => todo!()
                    };

                    match deserialized_pdu {
                        Some(DeserializedPdu::AssociationRequest(pdu)) => {
                            info!("Sending event");
                            event_tx.send(
                                Event::AssociateRequestPdu(pdu)
                            ).await;
                        },
                        _ => {
                            return Err("test".into())
                        }
                    };
                },
                Err(e) => {
                    return Err("test".into())
                }
            }
        }
    });

    match read_task.await.unwrap() {
        Ok(_) => {
            info!("exited gracefully");
        },
        Err(e) => {
            cancel.cancel();
            info!(
                "Read error: {}",
                e
            );
        }
    };

    tokio::join!(command_task, event_task);

    info!(
        "Closing connection: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    Ok(())
}

async fn handle_event_task(
    mut event_rx: mpsc::Receiver<Event>,
    mut command_tx: mpsc::Sender<Command>,
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
                info!("Event received: {:?}", event);

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

async fn handle_command_task<W, U: UpperLayerServiceUser>(
    mut writer: W,
    mut rx: mpsc::Receiver<Command>,
    mut event_tx: mpsc::Sender<Event>,
    mut user_connection: UpperLayerServiceProviderConnection<U>,
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

                info!("Command received");
                match command {
                    Command::AssociateAcceptPdu(resp) => {
                        handle_association_response(
                            AssociateRequestResponse::Accepted(resp),
                            &mut writer,
                        )
                        .await;
                    },

                    Command::AssociationIndication(indication) => {
                        user_connection.handle_associate_request(indication, &mut event_tx).await;
                    }

                    _ => todo!()
                };
            } => {}
        }
    }

    Ok(())
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

async fn handle_association_response<W>(response: AssociateRequestResponse, tcp: &mut W) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    match response {
        AssociateRequestResponse::Accepted(inner) => {
            info!("Sending A-ASSOCIATION-AC PDU: {:?}", inner);

            let pdu = AssociateRqAcPdu::from_response(&inner)?;
            tcp
                .write_all(serialize_association_pdu(&pdu)?.as_slice())
                .await?;
            Ok(())
        }
        AssociateRequestResponse::Rejected(inner) => {
            info!("Sending A-ASSOCIATION-RJ PDU: {:?}", inner);
            todo!();
        }
    }
}
