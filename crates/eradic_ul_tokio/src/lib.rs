mod artim;
mod handle_client;

use eradic_common::ul::associate::{AssociateRqAcPdu};
use eradic_common::ul::connection::UpperLayerStateMachineError;
use eradic_common::ul::event::{Command};
use eradic_common::ul::service::{AssociateRequestIndication};
use eradic_common::ul::{associate::PduDeserializationError, connection::{UpperLayerConnection}, event::{Event, Indication}};

use thiserror::Error;

use core::net::SocketAddr;
use std::{io::Cursor};

use tokio::net::{TcpStream};
use tracing::{instrument};

use crate::handle_client::handle_client;

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
    #[error(transparent)]
    UpperLayerStateMachineError(#[from] UpperLayerStateMachineError),
}

#[instrument(skip(tcp, scu_handler) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub async fn acceptor_handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    scu_handler: F,
) -> Result<(), HandleClientError>
where
    F: Fn(Indication) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<Event>> + Send + 'static,
{
    let connection = UpperLayerConnection::new_no_assoc_with_addresses(
        tcp.local_addr()?.ip(),
        tcp.local_addr()?.port(),
        socket_addr.ip(),
        socket_addr.port(),
    );

    handle_client(tcp, socket_addr, connection, scu_handler, vec![
        Event::TransportConnectionIndication
    ]).await
}

#[instrument(skip(tcp, scu_handler) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub async fn requestor_handle_client<F, Fut>(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    request: AssociateRequestIndication,
    scu_handler: F
) -> Result<(), HandleClientError>
where
    F: Fn(Indication) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<Event>> + Send + 'static,
{
    let connection = UpperLayerConnection::new_client(
        tcp.local_addr()?.ip(),
        tcp.local_addr()?.port(),
        socket_addr.ip(),
        socket_addr.port(),
    );

    handle_client(tcp, socket_addr, connection, scu_handler, vec![
        Event::AssociateRequestPrimitive(request)
    ]).await
}
