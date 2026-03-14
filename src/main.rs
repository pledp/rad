mod service_user;

use core::net::SocketAddr;
use std::io::Cursor;
use std::string::String;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, atomic::{AtomicI64, Ordering}};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use rad_common::{
    associate::{
        AssociateRqAcPdu, deserialize_association_pdu,
        rj::{RejectReason, AcseReason, PresentationReason, RejectSource, RejectResult}
    },
    pdu::{PduHeader, PduType, read_pdu_header}
};


use eradic_adaptor::{AssociationResult, UpperLayerServiceUser};

use crate::{
    service_user::ServiceUser,
};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:104").await?;
    println!("Listening for connections...");

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

async fn handle_client<U: UpperLayerServiceUser>(
    mut tcp: TcpStream,
    mut socket_addr: SocketAddr,
    service_user: Arc<Mutex<U>>)
-> Result<()> {
    println!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let conn = Connection::from_tcp_stream(tcp, service_user).listen_for_request().await?;

    Ok(())
}

struct Established {}
struct Closing {}

/// DICOM upper layer connection state 2 (Sta2).
///
/// Waiting for A-ASSOCIATE-RQ PDU from client.
struct Waiting<U: UpperLayerServiceUser> {
    service_user: Arc<Mutex<U>>
}

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
struct Connection<S> {
    stream: TcpStream,
    state_data: S,
}

impl<U: UpperLayerServiceUser> Connection<Waiting<U>> {
    pub fn from_tcp_stream(stream: TcpStream, service_user: Arc<Mutex<U>>) -> Self {
        Self {
            stream,
            state_data: Waiting {service_user},
        }
    }

    /// Wait for incoming A-ASSOCIATE-RQ PDU.
    pub async fn listen_for_request(mut self) -> Result<AssociationResult> {
        // Read DICOM header
        let mut buffer = [0u8; 6];

        let n = self.stream.peek(&mut buffer).await?;

        let mut cursor = Cursor::new(buffer);
        let header = read_pdu_header(&mut cursor)?;

        match header.pdu_type {
            PduType::AssociateRequest => {
                // Read entire PDU
                let mut buffer = vec![0u8; n + header.length as usize];

                let n = self.stream.read_exact(&mut buffer).await?;

                let mut cursor = Cursor::new(buffer);
                let pdu = deserialize_association_pdu(&mut cursor)?;

                service_provider_rq_pdu_validation(&pdu);

                {
                    let mut guard = self.state_data.service_user.lock().unwrap();
                    guard.handle_associate_request(pdu);
                }

                todo!()
            }
            _ => return Err("Invalid PDU type".into()),
        }
    }
}

fn service_provider_rq_pdu_validation(pdu: &AssociateRqAcPdu) -> Result<Option<AssociationResult>> {
    let source = RejectSource::Acse;

    if pdu.protocol_version != 1 {
        return Ok(Some(AssociationResult::Rejected {
            result: RejectResult::Transient,
            source,
            reason: RejectReason::Acse(AcseReason::ProtocolNotSupported)
        }))
    }

    todo!();
}
