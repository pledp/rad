mod pacs;

use core::net::SocketAddr;
use std::sync::Arc;
use std::io::{ Cursor };
use std::collections::HashMap;
use std::string::String;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream}
};

use rad_common::{
    pdu::{PduHeader, PduType, read_pdu_header},
    associate::deserialize_association_pdu,
};

use crate::pacs::Pacs;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

trait ApplicationEntity {
    fn handle_associate_request(&self) -> AssociateResult;
}

#[tokio::main]
async fn main() -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:104").await?;
    println!("Listening for connections...");

    let mut application_entities: HashMap<String, Box<dyn ApplicationEntity>> = HashMap::new();
    application_entities.insert("rad".into(), Box::new(Pacs {}));

    let application_registry: Arc<HashMap<String, Box<dyn ApplicationEntity>>> = Arc::new(application_entities);

    loop {
        let (mut tcp, mut socket_addr) = server.accept().await?;
        tokio::spawn(handle_client(tcp, socket_addr));
    }

    Ok(())
}

async fn handle_client(mut tcp: TcpStream, mut socket_addr: SocketAddr) -> Result<()> {
    println!("Connected client: {}:{}", socket_addr.ip(), socket_addr.port());

    let conn = Connection::from_stream(tcp)
        .listen_for_request().await?;


    Ok(())
}

enum AssociateResult {
    Accepted,
    Rejected,
}

struct Established {}
struct Closing {}

/// DICOM upper layer connection state 2 (Sta2).
///
/// Waiting for A-ASSOCIATE-RQ PDU from client.
struct Waiting {}

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
struct Connection<S = Waiting> {
    stream: TcpStream,
    state_data: S,
}

impl Connection<Waiting> {
    pub fn from_stream(stream: TcpStream) -> Self {
        Self {
            stream,
            state_data: Waiting {},
        }
    }

    /// Wait for incoming A-ASSOCIATE-RQ PDU.
    pub async fn listen_for_request(mut self) -> Result<AssociateResult> {
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
                let pdu = deserialize_association_pdu(&mut cursor);
            }
            _ => {
                return Err("Invalid PDU type".into())
            }
        };

        Ok(AssociateResult::Accepted)
    }
}
