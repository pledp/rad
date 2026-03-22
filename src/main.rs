mod service_user;

use core::net::SocketAddr;
use std::io::Cursor;
use std::net::IpAddr;
use std::string::String;
use std::collections::HashMap;
use std::sync::{Arc, atomic::{AtomicI64, Ordering}};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use rad_common::{
    pdu::{PduType, read_pdu_header},
    associate::AssociationResult,
};


use eradic_adaptor::{UpperLayerServiceUser, issue_indication};

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

    let conn = UpperLayerConnection::from_tcp_stream(tcp, socket_addr.ip(), service_user).listen_for_request().await;

    Ok(())
}

struct Established {}
struct Closing {}

/// DICOM upper layer connection state 2 (Sta2).
///
/// Waiting for A-ASSOCIATE-RQ PDU from client.
struct Waiting {
    client_address: IpAddr
}

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
struct UpperLayerConnection<S, U: UpperLayerServiceUser> {
    stream: TcpStream,
    service_user: Arc<Mutex<U>>,
    state_data: S,
}

impl<U: UpperLayerServiceUser> UpperLayerConnection<Waiting, U> {
    pub fn from_tcp_stream(stream: TcpStream, mut client_address: IpAddr, service_user: Arc<Mutex<U>>) -> Self {
        Self {
            stream,
            service_user,
            state_data: Waiting {client_address},
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

                let indication = issue_indication(
                    &mut cursor,
                    self.state_data.client_address,
                    self.stream.local_addr()?.ip()
                )?;

                {
                    let mut guard = self.service_user.lock().await;

                    guard.handle_associate_request(indication).await;
                }

                todo!()
            }
            _ => return Err("Invalid PDU type".into()),
        }
    }
}
