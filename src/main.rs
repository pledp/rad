mod service_user;

use core::net::SocketAddr;
use std::collections::HashMap;
use std::io::{Cursor, BufReader};
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use eradic_adaptor::PduCommand;
use rad_common::associate::{AssociateRqAcPdu, serialize_association_pdu};
use rad_common::pdu::{PDU_HEADER_LENGTH, PduHeader, read_pdu_header};

use rad_common::service::AssociateRequestResponse;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use eradic_adaptor::{
    UpperLayerServiceUserAsync, association::UpperLayerConnection, handle_incoming_pdu_async,
};

use crate::service_user::ServiceUser;

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

async fn handle_client<U: UpperLayerServiceUserAsync>(
    mut tcp: TcpStream,
    mut socket_addr: SocketAddr,
    service_user: Arc<Mutex<U>>,
) -> Result<()> {
    println!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let mut conn = UpperLayerConnection::new();

    loop {
        let header = tokio_read_pdu_header(&mut tcp).await?;
        let mut buffer = vec![0u8; PDU_HEADER_LENGTH + header.length as usize];
        let n = tcp.read_exact(&mut buffer).await?;

        let mut cursor = Cursor::new(buffer);

        let mut guard = service_user.lock().await;
        let (new_conn, command) = handle_incoming_pdu_async(
            &mut cursor,
            conn,
            &mut *guard,
            socket_addr.ip(),
            tcp.local_addr()?.ip(),
        )
        .await?;

        match command {
            PduCommand::AssociationResponse(response) => {
                handle_association_response(response, &mut tcp).await?;
            }
        }

        conn = new_conn;
    }
}

async fn tokio_read_pdu_header(tcp: &mut TcpStream) -> Result<PduHeader> {
    let mut buffer = [0u8; 6];

    let n = tcp.peek(&mut buffer).await?;

    let mut cursor = Cursor::new(buffer);
    read_pdu_header(&mut cursor)
}

async fn handle_association_response(response: AssociateRequestResponse, tcp: &mut TcpStream) -> Result<()> {
    match response {
        AssociateRequestResponse::Accepted(inner) => {
            let pdu = AssociateRqAcPdu::new_ac(&inner.called_ae, &inner.calling_ae)?;

            tcp
                .write_all(serialize_association_pdu(&pdu)?.as_slice())
                .await?;
            todo!();
        }
        AssociateRequestResponse::Rejected(inner) => {
            println!("test");
            todo!();
        }
    }
}
