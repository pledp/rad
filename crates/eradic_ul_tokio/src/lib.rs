mod artim;
mod handle_client;

use eradic_common::ul::associate::{AssociateRqAcPdu};
use eradic_common::ul::connection::UpperLayerStateMachineError;
use eradic_common::ul::event::{Command, Request, ServiceUserToServiceProvider};
use eradic_common::ul::service::{AssociateRequestIndication};
use eradic_common::ul::{associate::PduDeserializationError, connection::{UpperLayerConnection}, event::{Event, ServiceProviderToServiceUser}};

use thiserror::Error;
use tokio::sync::mpsc;
use tracing_log::log::info;

use core::net::SocketAddr;

use tokio::net::{TcpStream};
use tracing::{instrument};

use crate::handle_client::handle_connection;

pub struct UpperLayerHandle {
    pub scu_to_scp_tx: mpsc::Sender<ServiceUserToServiceProvider>,
    pub scp_to_scu_rx: mpsc::Receiver<ServiceProviderToServiceUser>
}

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
    #[error(transparent)]
    UpperLayerStateMachineError(#[from] UpperLayerStateMachineError),
}

#[instrument(skip(tcp) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub fn acceptor_handle_client(
    tcp: TcpStream,
    socket_addr: SocketAddr,
) -> Result<UpperLayerHandle, HandleClientError>
{
    info!(
        "Connected client: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    let connection = UpperLayerConnection::new_no_assoc_with_addresses(
        tcp.local_addr()?.ip(),
        tcp.local_addr()?.port(),
        socket_addr.ip(),
        socket_addr.port(),
    )?;

    let (scu_to_scp_tx, scu_to_scp_rx) = mpsc::channel(32);
    let (scp_to_scu_tx, scp_to_scu_rx) = mpsc::channel(32);

    tokio::spawn(handle_connection(
        tcp,
        socket_addr,
        connection,
        scp_to_scu_tx,
        scu_to_scp_rx,
        vec![
            Event::TransportConnectionIndication
        ]
    ));

    Ok(UpperLayerHandle { scu_to_scp_tx, scp_to_scu_rx })
}

#[instrument(skip(tcp, request) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub fn requestor_handle_connection(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    request: AssociateRequestIndication,
) -> Result<UpperLayerHandle, HandleClientError>
{

    let ip = tcp.local_addr()?.ip();
    let port = tcp.local_addr()?.port();

    let connection = UpperLayerConnection::new_client(
        ip,
        port,
        socket_addr.ip(),
        socket_addr.port(),
    )?;

    let (scu_to_scp_tx, scu_to_scp_rx) = mpsc::channel(32);
    let (scp_to_scu_tx, scp_to_scu_rx) = mpsc::channel(32);

    tokio::spawn(handle_connection(
        tcp,
        socket_addr,
        connection,
        scp_to_scu_tx,
        scu_to_scp_rx,
        vec![
            Event::ConnectionOpen(request),
        ]
    ));

    Ok(UpperLayerHandle { scu_to_scp_tx, scp_to_scu_rx })
}
