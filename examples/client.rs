use eradic::ul::connection::format_presentation_address;
use eradic::ul::associate::{MaximumLength, UserInformation};
use eradic::ul::event::{Event, ServiceProviderToServiceUser, Request};
use eradic::ul::service::{AssociateRequestIndication, PresentationContextDefinitionListBuilder};
use eradic_ul_tokio::requestor_handle_connection;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::fmt;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    fmt().with_max_level(tracing::Level::DEBUG).init();

    info!("Connecting to 127.0.0.1:104");
    let stream = TcpStream::connect("127.0.0.1:104").await?;
    info!("Connected to {}", stream.peer_addr()?);

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
        move |indication: ServiceProviderToServiceUser| async move {
            info!("indication received: {}", <&str>::from(&indication));
        }
    };

    let socket_addr = stream.peer_addr()?;
    let (_, scu_rx) = mpsc::channel::<Request>(32);

    let result = requestor_handle_connection(stream, socket_addr, indication, scu_handler, scu_rx).await;
    if let Err(ref e) = result {
        warn!("client exited with error: {e}");
    }

    Ok(())
}
