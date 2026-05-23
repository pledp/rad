use std::path::{Path};

use eradic::ul::connection::{format_presentation_address};
use eradic::ul::associate::abort::{AssociateAbortPdu, serialize_abort_pdu};
use eradic_ul_tokio::requestor_handle_client;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

use eradic::ul::associate::{
    AssociateRqAcPdu, MaximumLength, UserInformation, serialize_associate_pdu,
};
use eradic::ul::event::{Event, Indication};
use eradic::open_file;
use eradic::ul::service::{
    AssociateRequestIndication, PresentationContextDefinitionListBuilder,
};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {

    let mut stream = TcpStream::connect("127.0.0.1:104").await?;
    println!("Connected to server");

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


    let scu_handler = {
        move |indication: Indication| async move {
            println!("Indication!");
            None::<Event>
        }
    };

    let socket_addr = stream.peer_addr()?;

    loop {}

    Ok(())
}
