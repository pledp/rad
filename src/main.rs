mod adapter;
mod pacs;

use core::net::SocketAddr;
use std::collections::HashMap;
use std::io::Cursor;
use std::string::String;
use std::sync::Arc;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use rad_common::{
    associate::deserialize_association_pdu,
    pdu::{PduHeader, PduType, read_pdu_header},
};

use crate::{
    adapter::{ApplicationEntity, AssociationResult},
    pacs::Pacs
};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

type ApplicationEntityRegistry = Arc<HashMap<String, Box<dyn ApplicationEntity>>>;

#[tokio::main]
async fn main() -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:104").await?;
    println!("Listening for connections...");

    let mut application_entities: HashMap<String, Box<dyn ApplicationEntity>> = HashMap::new();
    application_entities.insert("rad".into(), Box::new(Pacs {}));

    let application_registry: ApplicationEntityRegistry =
        Arc::new(application_entities);

    loop {
        let (mut tcp, mut socket_addr) = server.accept().await?;

        let registry = Arc::clone(&application_registry);

        tokio::spawn(handle_client(tcp, socket_addr, registry));
    }

    Ok(())
}

async fn handle_client(mut tcp: TcpStream, mut socket_addr: SocketAddr, registry: ApplicationEntityRegistry) -> Result<()> {
    println!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let conn = Connection::from_tcp_stream(tcp, registry).listen_for_request().await?;

    Ok(())
}

struct Established {}
struct Closing {}

/// DICOM upper layer connection state 2 (Sta2).
///
/// Waiting for A-ASSOCIATE-RQ PDU from client.
struct Waiting {
    registry: ApplicationEntityRegistry
}

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
struct Connection<S> {
    stream: TcpStream,
    state_data: S,
}

impl Connection<Waiting> {
    pub fn from_tcp_stream(stream: TcpStream, registry: ApplicationEntityRegistry) -> Self {
        Self {
            stream,
            state_data: Waiting {registry},
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

                let registry = &self.state_data.registry;

                Ok(AssociationResult::Accepted)
            }
            _ => return Err("Invalid PDU type".into()),
        }
    }
}
