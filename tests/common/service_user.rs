use std::future::Future;

use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use eradic::ul::event::{ServiceProviderToServiceUser, ServiceUserToServiceProvider};
use eradic_ul_tokio::{HandleClientError, acceptor_handle_client};

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub async fn create_server_one_connection<F, Fut>(
    server: TcpListener,
    handler: F,
) -> Result<()>
where
    F: Fn(ServiceProviderToServiceUser, mpsc::Sender<ServiceUserToServiceProvider>) -> Fut
        + Send
        + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let _: JoinHandle<std::result::Result<(), HandleClientError>> = tokio::spawn(async move {
        let (tcp, socket_addr) = server.accept().await?;
        let mut handle = acceptor_handle_client(tcp, socket_addr)?;

        while let Some(indication) = handle.scp_to_scu_rx.recv().await {
            handler(indication, handle.scu_to_scp_tx.clone()).await;
        }

        Ok(())
    });

    Ok(())
}
