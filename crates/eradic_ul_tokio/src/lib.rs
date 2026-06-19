mod artim;
mod handle_client;

use eradic_common::ul::associate::rq_ac::AssociateRqAcPdu;
use eradic_common::ul::connection::{StateTransition, UpperLayerConnectionState, UpperLayerStateMachineError};
use eradic_common::ul::event::{Command, Request, ServiceUserToServiceProvider};
use eradic_common::ul::service::{AssociateRequestIndicationPrimitive};
use eradic_common::ul::{associate::PduDeserializationError, connection::{UpperLayerConnection}, event::{Event, ServiceProviderToServiceUser}};

use thiserror::Error;
#[cfg(feature = "state-watch")]
use tokio::sync::watch;
use tokio::sync::mpsc;
use tracing_log::log::info;

use core::net::SocketAddr;

use tokio::net::{TcpStream};
use tokio::task::JoinHandle;
use tracing::{instrument};

use crate::handle_client::{StateSenders, handle_connection};

/// Handle for various resources related to the association, including the Tokio task and channels for communication.
pub struct UpperLayerHandle {
    pub scu_to_scp_tx: mpsc::Sender<ServiceUserToServiceProvider>,
    pub scp_to_scu_rx: mpsc::Receiver<ServiceProviderToServiceUser>,
    pub task: JoinHandle<Result<(), HandleClientError>>,

    /// Synchronous, point-in-time access to the current state via `.borrow()`. Lossy: fast
    /// back-to-back transitions can be coalesced, so intermediate states may be skipped.
    #[cfg(feature = "state-watch")]
    pub state_watch: watch::Receiver<StateTransition>,
    /// Every transition, in order, with none dropped. Use this when exact history matters.
    #[cfg(feature = "state-mpsc")]
    pub state_mpsc: mpsc::Receiver<StateTransition>,
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

    #[cfg(feature = "state-watch")]
    let (state_watch_tx, state_watch_rx) = watch::channel(StateTransition {
        event: None,
        state: UpperLayerConnectionState::Idle,
    });
    #[cfg(feature = "state-mpsc")]
    let (state_mpsc_tx, state_mpsc_rx) = mpsc::channel(32);

    let state_senders = StateSenders {
        #[cfg(feature = "state-watch")]
        watch: state_watch_tx,
        #[cfg(feature = "state-mpsc")]
        mpsc: state_mpsc_tx,
    };

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
        ],
        state_senders,
    ));

    Ok(UpperLayerHandle {
        scu_to_scp_tx,
        scp_to_scu_rx,
        #[cfg(feature = "state-watch")]
        state_watch: state_watch_rx,
        #[cfg(feature = "state-mpsc")]
        state_mpsc: state_mpsc_rx,
        task,
    })
}

#[instrument(skip(tcp, request) fields(ip = %socket_addr.ip(), port = %socket_addr.port()))]
pub fn requestor_handle_connection(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    request: AssociateRequestIndicationPrimitive,
) -> Result<UpperLayerHandle, HandleClientError>
{

    let ip = tcp.local_addr()?.ip();
    let port = tcp.local_addr()?.port();

    let connection = UpperLayerConnection::new()?;

    let (scu_to_scp_tx, scu_to_scp_rx) = mpsc::channel(32);
    let (scp_to_scu_tx, scp_to_scu_rx) = mpsc::channel(32);

    #[cfg(feature = "state-watch")]
    let (state_watch_tx, state_watch_rx) = watch::channel(StateTransition {
        event: None,
        state: UpperLayerConnectionState::Idle,
    });
    #[cfg(feature = "state-mpsc")]
    let (state_mpsc_tx, state_mpsc_rx) = mpsc::channel(32);

    let state_senders = StateSenders {
        #[cfg(feature = "state-watch")]
        watch: state_watch_tx,
        #[cfg(feature = "state-mpsc")]
        mpsc: state_mpsc_tx,
    };

    let task = tokio::spawn(handle_connection(
        tcp,
        socket_addr,
        connection,
        scp_to_scu_tx,
        scu_to_scp_rx,
        vec![
            Event::AssociateRequestPrimitive(request),
            Event::TransportConnectionConfirmation {
                called_address: ip,
                called_port: port,
                calling_address: socket_addr.ip(),
                calling_port: socket_addr.port(),
            },
        ],
        state_senders,
    ));

    Ok(UpperLayerHandle {
        scu_to_scp_tx,
        scp_to_scu_rx,
        #[cfg(feature = "state-watch")]
        state_watch: state_watch_rx,
        #[cfg(feature = "state-mpsc")]
        state_mpsc: state_mpsc_rx,
        task,
    })
}
