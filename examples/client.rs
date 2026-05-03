use std::path::{Path};

use eradic::ul::connection::{UpperLayerRequestorConnection, format_presentation_address, handle_client_event};
use eradic::ul::associate::abort::{AssociateAbortPdu, serialize_abort_pdu};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

use eradic::ul::associate::{
    AssociateRqAcPdu, MaximumLength, UserInformation, serialize_associate_pdu,
};
use eradic::ul::event::Event;
use eradic::open_file;
use eradic::ul::service::{
    AssociateRequestIndication, PresentationContextDefinitionListBuilder,
};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let _file = open_file(Path::new("assets/sample.dcm"))?;

    let mut stream = TcpStream::connect("127.0.0.1:104").await?;
    println!("Connected to server");

    let mut conn = UpperLayerRequestorConnection::new_client();

    // TODO: Builder
    let indication = AssociateRequestIndication::new(
        "1.2.840.10008.3.1.1.1".into(),
        "rad".into(),
        "test1".into(),
        vec![UserInformation::MaximumLength(MaximumLength {
            maximum_length: 300,
        })],
        format_presentation_address(stream.local_addr()?.ip(), stream.local_addr()?.port()),
        format_presentation_address(stream.peer_addr()?.ip(), stream.peer_addr()?.port()),
        vec![
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840.10008.1.1".to_string())
                .add_transfer_syntax("1.2.840.10008.1.2".to_string())
                .build()?,
        ],
    );

    let (conn, command) = handle_client_event(conn, Event::ConnectionOpen)?;

    let pdu = AssociateRqAcPdu::try_from(indication)?;

    send_rq(&mut stream, pdu).await;

    Ok(())
}

async fn send_rq(tcp: &mut TcpStream, pdu: AssociateRqAcPdu) -> Result<()> {
    let mut writer = BufWriter::new(tcp);
    let abort = AssociateAbortPdu::new(
        eradic::ul::associate::abort::AbortSource::ServiceProvider,
        eradic::ul::associate::abort::AbortReason::NoReason,
    );

    writer
        .write_all(serialize_associate_pdu(&pdu).as_slice())
        .await?;
    writer.flush().await?;


    let mut buffer = vec![0; 1024];
    let n = writer.read(&mut buffer).await?;
    println!("Server replied: {}", String::from_utf8_lossy(&buffer[..n]));


    writer
        .write_all(serialize_abort_pdu(&abort).as_slice())
        .await?;
    writer.flush().await?;


    println!("Waiting for more packets");
    let mut buffer = vec![0; 1024];
    let _n = writer.read(&mut buffer).await?;
    Ok(())
}
