use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

use tokio::{
    io::{AsyncWriteExt, BufWriter, AsyncReadExt},
    net::TcpStream,
};

use eradic_common::associate::{
    AssociateRqAcPdu, MaximumLength, UserInformation, serialize_association_pdu,
};
use eradic_common::event::Event;
use eradic_common::open_file;
use eradic_common::service::{AssociateRequestIndication, PresentationContextDefinitionListBuilder};

use eradic_common::connection::UpperLayerAcceptorConnection;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let file = open_file(Path::new("assets/sample.dcm"))?;

    let mut stream = TcpStream::connect("127.0.0.1:104").await?;
    println!("Connected to server");

    let mut conn = UpperLayerAcceptorConnection::new_client();

    // TODO: Builder
    let indication = AssociateRequestIndication::new(
        // TODO: Application context enum?
        "1.2.840.10008.3.1.1.1".into(),
        "rad".into(),
        "test1".into(),
        vec![UserInformation::MaximumLength(MaximumLength {
            maximum_length: 300,
        })],
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
        vec![
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840.10008.1.1".to_string())
                .add_transfer_syntax("1.2.840.10008.1.2".to_string())
                .build()?,
        ],
    );

    conn.handle_event(Event::AssociateRequestPrimitive(indication.clone()));
    conn.handle_event(Event::ConnectionOpen);

    let pdu = AssociateRqAcPdu::from_indication(&indication)?;

    send_rq(&mut stream, pdu).await;

    Ok(())
}

async fn send_rq(tcp: &mut TcpStream, pdu: AssociateRqAcPdu) -> Result<()> {
    let mut writer = BufWriter::new(tcp);

    writer
        .write_all(serialize_association_pdu(&pdu)?.as_slice())
        .await?;
    writer.flush().await?;

    let mut buffer = vec![0; 1024];
    let n = writer.read(&mut buffer).await?;
    println!("Server replied: {}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}
