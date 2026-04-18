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

use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use eradic_common::associate::{
    self, AssociateRqAcPdu, deserialize_association_pdu, serialize_association_pdu
};
use eradic_common::connection::UpperLayerAcceptorConnection;
use eradic_common::event::{Command};
use eradic_common::pdu::{DeserializedPdu, PDU_HEADER_LENGTH, PduHeader, PduType, read_pdu_header};

use eradic_common::service::AssociateRequestResponse;

use eradic_adaptor::{
    UpperLayerServiceUserAsync, handle_incoming_pdu_async,
};

use crate::service_user::ServiceUser;

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

async fn handle_client<U: UpperLayerServiceUserAsync + Send + 'static>(
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

    let (tx, mut rx) = mpsc::channel::<DeserializedPdu>(32);

    let cancel = CancellationToken::new();
    let child = cancel.child_token();

    let mut connection_task = tokio::spawn(async move {
        handle_pdu_task(service_user, writer, rx, called, socket_addr.ip(), child).await;
    });

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
                        Some(pdu) => {
                            if tx.send(pdu).await.is_err() {
                                return Err("test".into())
                            }
                        },
                        None => {
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

    tokio::join!(connection_task);

    info!(
        "Closing connection: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    Ok(())
}

async fn handle_pdu_task<U: UpperLayerServiceUserAsync + Send + 'static, W>(
    service_user: Arc<Mutex<U>>,
    mut writer: W,
    mut rx: mpsc::Receiver<DeserializedPdu>,
    called: IpAddr,
    calling: IpAddr,
    cancel: CancellationToken,
) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut conn = UpperLayerAcceptorConnection::new_server(called, calling);

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("cancelled");
                break;
            }
            _ = async {
                let deserialized_pdu = match rx.recv().await {
                    Some(pdu) => pdu,
                    None => {
                        return
                    }
                };

                let service_user = service_user.clone();

                let mut guard = service_user.lock().await;

                let (command, new_state) = handle_incoming_pdu_async(
                    deserialized_pdu,
                    &conn,
                    &mut *guard,
                )
                .await.unwrap();

                conn = new_state;

                match command {
                    Some(Command::AssociateAcceptPdu(resp)) => {
                        handle_association_response(
                            AssociateRequestResponse::Accepted(resp),
                            &mut writer,
                        )
                        .await;
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
            let pdu = AssociateRqAcPdu::from_response(&inner)?;
            tcp
                .write_all(serialize_association_pdu(&pdu)?.as_slice())
                .await?;
            Ok(())
        }
        AssociateRequestResponse::Rejected(inner) => {
            todo!();
        }
    }
}
