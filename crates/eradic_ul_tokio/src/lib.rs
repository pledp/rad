mod handle_client;

use eradic_common::ul::associate::{AssociateRqAcPdu, AssociateRqAcPduError, deserialized_pdu_from_reader, serialize_associate_pdu};
use eradic_common::ul::event::{Command, event_from_deserialized_pdu};
use eradic_common::ul::service::{AssociateRequestIndication, AssociateRequestResponse};
use eradic_common::ul::{associate::PduDeserializationError, connection::{UpperLayerConnection}, event::{Event, Indication}, pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header}};

use thiserror::Error;

use core::net::SocketAddr;
use std::{io::Cursor};

use tokio::net::{TcpStream};
use tokio::sync::mpsc;
use tracing::{info, instrument};

use crate::handle_client::handle_client;

#[derive(Debug, Error)]
pub enum HandleClientError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    PduDeserializationError(#[from] PduDeserializationError),
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
    let connection = UpperLayerConnection::new_server(
        tcp.local_addr()?.ip(),
        tcp.local_addr()?.port(),
        socket_addr.ip(),
        socket_addr.port(),
    );

    handle_client(tcp, socket_addr, connection, scu_handler, vec![]).await
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
        Command::AssociateRequestPdu(AssociateRqAcPdu::try_from(request)?)
    ]).await
}
