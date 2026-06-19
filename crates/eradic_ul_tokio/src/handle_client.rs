use core::net::SocketAddr;
use std::io::Cursor;

use eradic_common::DeserializedPdu;
use eradic_common::ul::associate::abort::{AbortReason, serialize_abort_pdu};
use eradic_common::ul::associate::rj::serialize_reject_pdu;
use eradic_common::ul::associate::{AssociationResult, PduDeserializationError, deserialized_pdu_from_reader};
use eradic_common::ul::associate::rq_ac::serialize_associate_pdu;
use eradic_common::ul::associate::presentation_context::PresentationContextError;
use eradic_common::ul::associate::user_information::UserInfoItemError;
use eradic_common::ul::pdu::{PDU_HEADER_LENGTH, PduType, read_pdu_header};
use eradic_common::ul::service::ProviderAbortIndicationPrimitive;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::oneshot;
#[cfg(feature = "state-watch")]
use tokio::sync::watch;
use tokio::{
    net::TcpStream,
    sync::mpsc,
    task::JoinSet,
};

use tracing::{info, instrument, warn};

use eradic_common::ul::connection::{StateTransition, UpperLayerConnection, UpperLayerConnectionState, handle_event};
use eradic_common::ul::event::{Command, Confirmation, Event, Request, ServiceProviderToServiceUser, ServiceUserToServiceProvider, event_from_deserialized_pdu};

use crate::artim::artim_task;

use crate::{HandleClientError};

/// Bundles the optional state-observation senders so they can be threaded through as a
/// single, unconditional parameter regardless of which `state-*` features are enabled.
pub struct StateSenders {
    #[cfg(feature = "state-watch")]
    pub watch: watch::Sender<StateTransition>,
    #[cfg(feature = "state-mpsc")]
    pub mpsc: mpsc::Sender<StateTransition>,
}

#[instrument(skip_all)]
pub async fn handle_connection(
    tcp: TcpStream,
    socket_addr: SocketAddr,
    connection: UpperLayerConnection,
    scp_to_scu_tx: mpsc::Sender<ServiceProviderToServiceUser>,
    scu_to_scp_rx: mpsc::Receiver<ServiceUserToServiceProvider>,
    initial_events: Vec<Event>,
    state_senders: StateSenders,
) -> Result<(), HandleClientError> {
    let (reader, writer) = tcp.into_split();

    let (command_tx, command_rx) = mpsc::channel::<(Command, UpperLayerConnectionState)>(32);
    let (event_tx, event_rx) = mpsc::channel::<Event>(32);

    let mut set: JoinSet<Result<(), HandleClientError>> = JoinSet::new();

    set.spawn(scu_to_scp_task(scu_to_scp_rx, event_tx.clone()));

    set.spawn(handle_event_task(
        event_rx,
        command_tx.clone(),
        connection,
        state_senders,
    ));

    set.spawn(handle_command_task(writer, command_rx, event_tx.clone(), scp_to_scu_tx));
    set.spawn(pdu_read_task(reader, event_tx.clone()));

    for event in initial_events {
        event_tx.send(event).await;
    }

    drop(event_tx);

    let mut client_result: Result<(), HandleClientError> = Ok(());

    while let Some(result) = set.join_next().await {
        match result {
            Ok(Err(e)) => {
                warn!("Task exited unexpectedly: {:?}", e);
                client_result = Err(e);
            }
            _ => {
                info!("Task exited")
            }
        }
    }

    info!("Connection ended");

    client_result
}

#[instrument(skip_all)]
async fn scu_to_scp_task(
    mut rx: mpsc::Receiver<ServiceUserToServiceProvider>,
    event_tx: mpsc::Sender<Event>,
) -> Result<(), HandleClientError>
{
    loop {
        tokio::select! {
            request = rx.recv() => {
                let Some(item) = request else { break; };
                match item {
                    ServiceUserToServiceProvider::AbortRequest => {
                        event_tx.send(Event::AbortRequestPrimitive).await;
                    }
                    ServiceUserToServiceProvider::AssociateRequestPrimitive(request) => {
                        event_tx.send(Event::AssociateRequestPrimitive(request)).await;
                    }
                    ServiceUserToServiceProvider::AssociateResponsePrimitive(response) => {
                        match response.result() {
                            AssociationResult::Accepted => {
                                event_tx.send(Event::AcceptedAssociateResponsePrimitive(response)).await;
                            }
                            AssociationResult::RejectedPermanent | AssociationResult::RejectedTransient => {
                                event_tx.send(Event::RejectedAssociateResponsePrimitive(response)).await;
                            }
                        }
                    }
                }
            }
            _ = event_tx.closed() => {
                break;
            }
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn pdu_read_task<R>(mut reader: R, event_tx: mpsc::Sender<Event>) -> Result<(), HandleClientError>
where
    R: AsyncRead + Unpin,
{
    loop {
        tokio::select! {
            result = read_full_pdu(&mut reader) => {
                let (buf, pdu_type) = match result {
                    Ok(val) => val,
                    Err(ReadPduError::DeserializeError(e)) => {
                        info!("{e}");
                        let _ = event_tx.send(Event::UnrecognizedPdu).await;
                        return Err(HandleClientError::PduDeserializationError(e));
                    }
                    Err(ReadPduError::Io(e)) => {
                        info!("{e}");
                        let _ = event_tx.send(Event::TransportConnectionClosedIndication).await;
                        return Err(HandleClientError::Io(e));
                    }
                };
                info!("PDU received: {:?}", pdu_type);

                match deserialized_pdu_from_reader(&mut Cursor::new(buf), pdu_type) {
                    Ok(pdu) => {
                        let _ = event_tx.send(event_from_deserialized_pdu(pdu)).await;
                    }
                    Err(PduDeserializationError::UnrecognizedItemType(item_type)) => {
                        let _ = event_tx.send(Event::UnrecognizedPduParameter).await;

                        return Err(HandleClientError::PduDeserializationError(
                            PduDeserializationError::UnrecognizedItemType(item_type)
                        ));
                    }
                    Err(e @ PduDeserializationError::InvalidUserInfoItem(UserInfoItemError::UnexpectedItemType(_)))
                    | Err(e @ PduDeserializationError::InvalidPresentationItem(PresentationContextError::UnexpectedItemType(_))) => {
                        let _ = event_tx.send(Event::UnexpectedPduParameter).await;

                        return Err(HandleClientError::PduDeserializationError(e));
                    }
                    Err(e) => {
                        let _ = event_tx.send(Event::InvalidPduParameter).await;

                        return Err(HandleClientError::PduDeserializationError(e));
                    }
                }
            }
            _ = event_tx.closed() => {
                return Ok(())
            }
        }
    }
}

#[instrument(skip_all)]
async fn handle_event_task(
    mut event_rx: mpsc::Receiver<Event>,
    command_tx: mpsc::Sender<(Command, UpperLayerConnectionState)>,
    mut conn: UpperLayerConnection,
    state_senders: StateSenders,
) -> Result<(), HandleClientError> {
    #[cfg(feature = "state-mpsc")]
    let _ = state_senders.mpsc.send(StateTransition { event: None, state: conn.state }).await;

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else { break; };
                info!("Received event: {}", <&str>::from(&event));

                let state_before = conn.state;

                #[cfg(any(feature = "state-watch", feature = "state-mpsc"))]
                let event_for_state = event.clone();

                match handle_event(conn, event) {
                    Ok((new_conn, commands)) => {
                        conn = new_conn;

                        #[cfg(feature = "state-watch")]
                        let _ = state_senders.watch.send(StateTransition {
                            event: Some(event_for_state.clone()),
                            state: conn.state,
                        });

                        #[cfg(feature = "state-mpsc")]
                        let _ = state_senders.mpsc.send(StateTransition {
                            event: Some(event_for_state.clone()),
                            state: conn.state,
                        }).await;

                        for cmd in commands {
                            let _ = command_tx.send((cmd, conn.state)).await;
                        }
                    }
                    Err(e) => {
                        let _ = command_tx.send((
                            Command::ProviderAbortIndicationPrimitive(
                                ProviderAbortIndicationPrimitive::new(AbortReason::NoReason)
                            ),
                            state_before
                        )).await;

                        return Err(e.into());
                    }
                }
            }
            _ = command_tx.closed() => {
                break;
            }
        }
    }

    info!("Closing event task");
    Ok(())
}

#[instrument(skip_all)]
async fn handle_command_task<W>(
    mut writer: W,
    mut rx: mpsc::Receiver<(Command, UpperLayerConnectionState)>,
    event_tx: mpsc::Sender<Event>,
    scp_to_scu_tx: mpsc::Sender<ServiceProviderToServiceUser>
) -> Result<(), HandleClientError>
where
    W: AsyncWrite + Unpin,
{
    let mut artim_cancel: Option<oneshot::Sender<()>> = None;

    while let Some((command, state)) = rx.recv().await {
        info!("Received command: {}", <&str>::from(&command));

        match command {
            // Acceptor and Requestor commands
            Command::AbortIndicationPrimitive(indication) => {
                scp_to_scu_tx
                    .send(ServiceProviderToServiceUser::AbortIndicationPrimitive(indication))
                    .await;

                writer.shutdown().await;
                return Ok(());
            }

            Command::ProviderAbortIndicationPrimitive(indication) => {
                scp_to_scu_tx
                    .send(ServiceProviderToServiceUser::ProviderAbortIndicationPrimitive(indication))
                    .await;
            }

            Command::StartArtimTimer => {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                tokio::spawn(artim_task(cancel_rx, event_tx.clone()));
                artim_cancel = Some(cancel_tx);

                info!("ARTIM timer started");
            }

            Command::StopArtimTimer => {
                artim_cancel = None;

                info!("ARTIM timer stopped")
            }

            Command::CloseConnection => {
                writer.shutdown().await;

                return Ok(());
            }

            // Acceptor commands
            Command::AssociateAcceptPdu(pdu) => {
                stream_write_pdu(DeserializedPdu::AssociateAccept(pdu), &mut writer).await;
            }

            Command::AssociateRejectPdu(pdu) => {
                stream_write_pdu(DeserializedPdu::AssociateReject(pdu), &mut writer).await;
            }

            Command::AssociateIndicationPrimitive(indication) => {
                scp_to_scu_tx
                    .send(ServiceProviderToServiceUser::AssociateIndicationPrimitive(indication))
                    .await;
            }

            // Requestor commands
            Command::AssociateRequestPdu(pdu) => {
                stream_write_pdu(DeserializedPdu::AssociateRequest(pdu), &mut writer).await;
            }

            Command::AssociateConfirmationPrimitive(prim) => {
                scp_to_scu_tx
                    .send(Confirmation::AssociateConfirmationPrimitive(prim))
                    .await;
            }

            _ => {}
        };

        if state == UpperLayerConnectionState::Idle {
            return Ok(())
        }
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum ReadPduError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    DeserializeError(#[from] PduDeserializationError)
}

#[instrument(skip_all)]
async fn read_full_pdu<R>(reader: &mut R) -> Result<(Vec<u8>, PduType), ReadPduError>
where
    R: AsyncRead + Unpin,
{
    let mut header_buf = [0u8; 6];
    reader.read_exact(&mut header_buf).await?;

    let mut cursor = Cursor::new(header_buf);
    let header = read_pdu_header(&mut cursor)?;
    info!("{:?}", header);

    let mut buffer = vec![0u8; PDU_HEADER_LENGTH + header.length as usize];
    buffer[..6].copy_from_slice(&header_buf);

    reader.read_exact(&mut buffer[6..]).await?;

    Ok((buffer, header.pdu_type))
}

#[instrument(skip_all)]
async fn stream_write_pdu<W>(response: DeserializedPdu, tcp: &mut W)
where
    W: AsyncWrite + Unpin,
{
    info!("Sending PDU: {}", <&str>::from(&response));

    match response {
        DeserializedPdu::AssociateRequest(pdu) |
        DeserializedPdu::AssociateAccept(pdu) => {
            tcp.write_all(serialize_associate_pdu(&pdu).as_slice()).await;
        }

        DeserializedPdu::Abort(pdu) => {
            tcp.write_all(serialize_abort_pdu(&pdu).as_slice()).await;
        }

        DeserializedPdu::AssociateReject(pdu) => {
            tcp.write_all(serialize_reject_pdu(&pdu).as_slice()).await;
        }

        _ => {
            todo!()
        }
    }
}
