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
use tokio::task::JoinHandle;
use tracing::{instrument};

use crate::handle_client::handle_connection;

pub struct UpperLayerHandle {
    pub scu_to_scp_tx: mpsc::Sender<ServiceUserToServiceProvider>,
    pub scp_to_scu_rx: mpsc::Receiver<ServiceProviderToServiceUser>,
    pub task: JoinHandle<Result<(), HandleClientError>>,
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

    let ip = tcp.local_addr()?.ip();
    let port = tcp.local_addr()?.port();

    let connection = UpperLayerConnection::new()?;

    let (scu_to_scp_tx, scu_to_scp_rx) = mpsc::channel(32);
    let (scp_to_scu_tx, scp_to_scu_rx) = mpsc::channel(32);

    let task = tokio::spawn(handle_connection(
        tcp,
        socket_addr,
        connection,
        scp_to_scu_tx,
        scu_to_scp_rx,
        vec![
            Event::TransportConnectionIndication {
                called_address: ip,
                called_port: port,
                calling_address: socket_addr.ip(),
                calling_port: socket_addr.port(),
            }
        ]
    ));

    Ok(UpperLayerHandle { scu_to_scp_tx, scp_to_scu_rx, task })
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

    let connection = UpperLayerConnection::new()?;

    let (scu_to_scp_tx, scu_to_scp_rx) = mpsc::channel(32);
    let (scp_to_scu_tx, scp_to_scu_rx) = mpsc::channel(32);

    let task = tokio::spawn(handle_connection(
        tcp,
        socket_addr,
        connection,
        scp_to_scu_tx,
        scu_to_scp_rx,
        vec![
            Event::AssociateRequest(request),
            Event::ConnectionOpen {
                called_address: ip,
                called_port: port,
                calling_address: socket_addr.ip(),
                calling_port: socket_addr.port(),
            },
        ]
    ));

    Ok(UpperLayerHandle { scu_to_scp_tx, scp_to_scu_rx, task })
}
