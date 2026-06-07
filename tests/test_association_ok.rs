mod service_user;

use tokio::net::{TcpListener, TcpStream};

use tokio::task::JoinHandle;
use tracing::{info};
use tracing_subscriber::{fmt};

use eradic::ul::{associate::{MaximumLength, UserInformation}, event::{ServiceProviderToServiceUser, ServiceUserToServiceProvider}, service::{AssociateRequest, PresentationContextDefinitionListBuilder}};
use eradic_ul_tokio::{HandleClientError, acceptor_handle_client, requestor_handle_connection};

use crate::service_user::LocalUpperLayerServiceUser;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;


async fn create_server_one_connection(server: TcpListener) -> Result<()> {
    let ul_scu = LocalUpperLayerServiceUser::new();

    let _: JoinHandle<std::result::Result<(), HandleClientError>> = tokio::spawn(async move {
        let (tcp, socket_addr) = server.accept().await?;
        let mut handle = acceptor_handle_client(tcp, socket_addr)?;

        while let Some(indication) = handle.scp_to_scu_rx.recv().await {
            match indication {
                ServiceProviderToServiceUser::AssociateIndicationPrimitive(indication) => {
                    handle.scu_to_scp_tx.send(
                        ServiceUserToServiceProvider::AssociateResponsePrimitive(ul_scu.handle_associate_request(indication).await)
                    ).await;
                }
                _ => {}
            }
        }

        Ok(())
    });

    Ok(())
}

#[tokio::test]
async fn test_association_client_ok() {
    fmt().with_max_level(tracing::Level::DEBUG).init();

    let server = TcpListener::bind("127.0.0.1:104").await.unwrap();
    create_server_one_connection(server).await;

    let stream = TcpStream::connect("127.0.0.1:104").await.unwrap();

    let request = AssociateRequest::new(
        "1.2.840.10008.3.1.1.1".into(),
        "rad".into(),
        "test1".into(),
        vec![UserInformation::MaximumLength(MaximumLength {
            maximum_length: 300,
        })],
        stream.local_addr().unwrap().ip(),
        stream.local_addr().unwrap().port(),
        stream.peer_addr().unwrap().ip(),
        stream.peer_addr().unwrap().port(),
        vec![
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840.10008.1.1".to_string())
                .add_transfer_syntax("1.2.840.10008.1.2".to_string())
                .build().unwrap(),
        ],
    );

    let socket_addr = stream.peer_addr().unwrap();
    let mut handle = requestor_handle_connection(stream, socket_addr, request).unwrap();

    let mut indications = vec![];
    while let Some(ind) = handle.scp_to_scu_rx.recv().await {
        indications.push(ind);
        handle.scu_to_scp_tx.send(ServiceUserToServiceProvider::AbortRequest).await;
    }

    let mut iter = indications.iter();
    assert!(matches!(iter.next(), Some(ServiceProviderToServiceUser::AssociateConfirmationPrimitive(_))));
    assert!(iter.next().is_none());

    handle.task.await;
}
