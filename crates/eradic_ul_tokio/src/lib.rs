mod artim;
mod handle_client;

use eradic_common::ul::associate::{AssociateRqAcPdu};
use eradic_common::ul::connection::UpperLayerStateMachineError;
use eradic_common::ul::event::{Command, Request};
use eradic_common::ul::service::{AssociateRequestIndication};
use eradic_common::ul::{associate::PduDeserializationError, connection::{UpperLayerConnection}, event::{Event, ServiceProviderToServiceUser}};

use thiserror::Error;
use tokio::sync::mpsc;
use tracing_log::log::info;

use core::net::SocketAddr;
use std::{io::Cursor};

use tokio::net::{TcpStream};
use tracing::{instrument};

use crate::handle_client::handle_connection;

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
    #[error(transparent)]
    UpperLayerStateMachineError(#[from] UpperLayerStateMachineError),
}

#[instrument(skip(tcp, scu_handler, scu_rx) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub async fn acceptor_handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    scu_handler: F,
    scu_rx: mpsc::Receiver<Request>,
) -> Result<(), HandleClientError>
where
    F: Fn(ServiceProviderToServiceUser) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
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
    );

    let result = handle_connection(
        tcp,
        socket_addr,
        connection,
        scu_handler,
        scu_rx,
        vec![
            Event::TransportConnectionIndication
        ]
    ).await;

    info!(
        "Closing connection: {}:{}",
        socket_addr.ip(),
        socket_addr.port()
    );

    result
}

#[instrument(skip(tcp, scu_handler, scu_rx, request) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub async fn requestor_handle_connection<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    request: AssociateRequestIndication,
    scu_handler: F,
    scu_rx: mpsc::Receiver<Request>,
) -> Result<(), HandleClientError>
where
    F: Fn(ServiceProviderToServiceUser) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{

    let ip = tcp.local_addr()?.ip();
    let port = tcp.local_addr()?.port();

    let connection = UpperLayerConnection::new_client(
        ip,
        port,
        socket_addr.ip(),
        socket_addr.port(),
    );

    let result = handle_connection(
        tcp,
        socket_addr,
        connection,
        scu_handler,
        scu_rx,
        vec![
            Event::ConnectionOpen(request),
        ]
    ).await;

    info!(
        "Connection closed: {}:{}",
        ip,
        port
    );

    result
}
