use std::{net::IpAddr, panic};
use ron::de::SpannedError;
use thiserror::Error;

use crate::ul::{
    associate::{
        abort::{AbortReason, AbortSource, AssociateAbortPdu}, rq_ac::{AssociateRqAcPdu, AssociateRqAcPduError}
    },
    event::{Command, CommandKind::{self, AssociateRejectPdu}, Event, EventKind},
    service::{AbortIndicationPrimitive, AcceptedAssociateConfirmationPrimitive, RejectedAssociateConfirmationPrimitive, AssociateRequestIndicationPrimitive, PrimitiveError, ProviderAbortIndicationPrimitive, associate_response_into_reject_pdu},
    table::TransitionTable,
};

use super::associate::rj::RejectSource::ServiceUser;

#[derive(Debug, Error)]
pub enum UpperLayerStateMachineError {
    #[error(transparent)]
    AssociateRqAcFromError(#[from] AssociateRqAcPduError),
    #[error(transparent)]
    PrimitiveError(#[from] PrimitiveError),
    #[error(transparent)]
    RonError(#[from] SpannedError),
    #[error("no transition for state={0:?} event={1:?}")]
    UnhandledEvent(UpperLayerConnectionState, EventKind),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Deserialize)]
pub enum UpperLayerConnectionState {
    #[serde(rename = "Sta1")]
    Idle,
    #[serde(rename = "Sta2")]
    WaitingForRequestPdu,
    #[serde(rename = "Sta3")]
    WaitingForResponsePrimitive,
    #[serde(rename = "Sta4")]
    WaitingForOpenConnection,
    #[serde(rename = "Sta5")]
    WaitingForAcRjPdu,
    #[serde(rename = "Sta6")]
    DataTransfer,
    #[serde(rename = "Sta13")]
    WaitForTcpClose,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StateTransition {
    /// The event that caused this transition. `None` for the initial state, which isn't the
    /// result of any event.
    pub event: Option<Event>,
    pub state: UpperLayerConnectionState,
}

fn abort_reason_from_event(event: &Event) -> AbortReason {
    match event {
        Event::UnrecognizedPdu => AbortReason::UnrecognizedPdu,
        Event::UnexpectedPdu => AbortReason::UnexpectedPdu,
        Event::UnrecognizedPduParameter => AbortReason::UnrecognizedPduParameter,
        Event::UnexpectedPduParameter => AbortReason::UnexpectedPduParameter,
        Event::InvalidPduParameter => AbortReason::InvalidPduParameter,
        _ => AbortReason::NoReason,
    }
}

/// DICOM upper layer connection. Transitions are defined in `transitions.ron`.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
pub struct UpperLayerConnection {
    pub state: UpperLayerConnectionState,
    request: Option<AssociateRequestIndicationPrimitive>,

    pub called_address: Option<IpAddr>,
    called_port: Option<u16>,
    calling_address: Option<IpAddr>,
    calling_port: Option<u16>,

    table: TransitionTable,
}

impl UpperLayerConnection {
    pub fn new() -> Result<Self, UpperLayerStateMachineError> {
        Ok(Self {
            state: UpperLayerConnectionState::Idle,
            request: None,
            called_address: None,
            called_port: None,
            calling_address: None,
            calling_port: None,
            table: TransitionTable::new()?,
        })
    }
}

/// Drives the DICOM UL state machine. Transitions are defined in `transitions.ron`.
pub fn handle_event(
    mut conn: UpperLayerConnection,
    event: Event,
) -> Result<(UpperLayerConnection, Vec<Command>), UpperLayerStateMachineError> {
    let state = conn.state;
    let event_kind = EventKind::from(&event);

    let entry = conn.table
        .lookup(state, event_kind)
        .ok_or(UpperLayerStateMachineError::UnhandledEvent(state, event_kind))?;

    // Clone to release the borrow on conn.table before mutating conn.state.
    let commands = entry.commands.clone();
    conn.state = entry.to;

    let abort_reason = abort_reason_from_event(&event);
    let mut out = Vec::new();

    match &event {
        Event::TransportConnectionConfirmation { called_address, called_port, calling_address, calling_port } => {
            conn.called_address = Some(*called_address);
            conn.called_port = Some(*called_port);
            conn.calling_address = Some(*calling_address);
            conn.calling_port = Some(*calling_port);
        }
        Event::TransportConnectionIndication { called_address, called_port, calling_address, calling_port } => {
            conn.called_address = Some(*called_address);
            conn.called_port = Some(*called_port);
            conn.calling_address = Some(*calling_address);
            conn.calling_port = Some(*calling_port);
        }
        Event::AssociateRequestPrimitive(indication) => {
            conn.request = Some(indication.clone());
        }
        _ => {}
    }

    let mut event = Some(event);

    for kind in &commands {
        let cmd = match kind {
            CommandKind::StartArtimTimer => Command::StartArtimTimer,
            CommandKind::StopArtimTimer  => Command::StopArtimTimer,
            CommandKind::CloseConnection => Command::CloseConnection,
            CommandKind::AbortPdu => Command::AbortPdu(AssociateAbortPdu::new(
                AbortSource::ServiceUser,
                abort_reason,
            )),
            CommandKind::ProviderAbortIndicationPrimitive => {
                Command::ProviderAbortIndicationPrimitive(ProviderAbortIndicationPrimitive::new(abort_reason))
            }

            CommandKind::AssociateIndicationPrimitive => {
                let Event::AssociateRequestPdu(pdu) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateIndicationPrimitive(AssociateRequestIndicationPrimitive::from_rq_pdu(
                    pdu,
                    format_presentation_address(conn.called_address.clone().unwrap(), conn.called_port.clone().unwrap()),
                    format_presentation_address(conn.calling_address.clone().unwrap(), conn.calling_port.clone().unwrap()),
                ))
            }
            CommandKind::AssociateAcceptPdu => {
                let Event::AcceptedAssociateResponsePrimitive(prim) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateAcceptPdu(AssociateRqAcPdu::try_from(prim)?)
            }
            CommandKind::AssociateRejectPdu => {
                let Event::RejectedAssociateResponsePrimitive(prim) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateRejectPdu(associate_response_into_reject_pdu(
                    prim,
                    ServiceUser
                ))
            }
            CommandKind::AssociateRequestPdu => {
                Command::AssociateRequestPdu(AssociateRqAcPdu::try_from(conn.request.clone().unwrap())?)
            }
            CommandKind::AbortIndicationPrimitive => {
                let Event::AssociateAbortPdu(pdu) = event.take().unwrap() else {
                    panic!()
                };

                Command::AbortIndicationPrimitive(AbortIndicationPrimitive::from_pdu(pdu))
            }
            CommandKind::AcceptedAssociateConfirmationPrimitive => {
                let Event::AssociateAcceptPdu(pdu) = event.take().unwrap() else {
                    panic!("unexpected event for AcceptedAssociateConfirmationPrimitive")
                };

                Command::AcceptedAssociateConfirmationPrimitive(AcceptedAssociateConfirmationPrimitive::from_ac_pdu(pdu)?)
            }
            CommandKind::RejectedAssociateConfirmationPrimitive => {
                let Event::AssociateRejectPdu(rj) = event.take().unwrap() else {
                    panic!("unexpected event for RejectedAssociateConfirmationPrimitive")
                };

                Command::RejectedAssociateConfirmationPrimitive(RejectedAssociateConfirmationPrimitive::from_rj_pdu(rj))
            }

            CommandKind::TransportConnectionRequest => {
                let Event::AssociateRequestPrimitive(request) = event.take().unwrap() else {
                    panic!()
                };

                Command::TransportConnectionRequest(request.called_address)
            }
        };

        out.push(cmd);
    }

    Ok((conn, out))
}

pub fn format_presentation_address(address: IpAddr, port: u16) -> String {
    format!("dicom:{address}:{port}")
}
