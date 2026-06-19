use tokio::net::TcpStream;
use tracing::{info};
use tracing_subscriber::fmt;

use eradic::ul::associate::user_information::{MaximumLength, UserInformation};
use eradic::ul::service::{AssociateRequest, PresentationContextDefinitionListBuilder};
use eradic_ul_tokio::requestor_handle_connection;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    fmt().with_max_level(tracing::Level::DEBUG).init();

    info!("Connecting to 127.0.0.1:104");
    let stream = TcpStream::connect("127.0.0.1:104").await?;
    info!("Connected to {}", stream.peer_addr()?);

    let request = AssociateRequest::new(
        "1.2.840.10008.3.1.1.1".into(),
        "rad".into(),
        "test1".into(),
        vec![UserInformation::MaximumLength(MaximumLength {
            maximum_length: 300,
        })],
        stream.local_addr()?.ip(),
        stream.local_addr()?.port(),
        stream.peer_addr()?.ip(),
        stream.peer_addr()?.port(),
        vec![
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840.10008.1.1".to_string())
                .add_transfer_syntax("1.2.840.10008.1.2".to_string())
                .build()?,
        ],
    );

    let socket_addr = stream.peer_addr()?;
    let mut handle = requestor_handle_connection(stream, socket_addr, request)?;

    while let Some(indication) = handle.scp_to_scu_rx.recv().await {
        info!("indication received: {}", <&str>::from(&indication));
    }

    handle.task.await;

    Ok(())
}
