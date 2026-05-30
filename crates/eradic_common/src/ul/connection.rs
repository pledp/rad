use std::{net::IpAddr, panic};
use ron::de::SpannedError;
use thiserror::Error;

use crate::ul::{
    associate::{
        AssociateRqAcPdu, AssociateRqAcPduError,
        abort::{AbortReason, AbortSource, AssociateAbortPdu},
    },
    event::{Command, CommandKind, Event, EventKind},
    service::{AbortIndication, AssociateConfirmation, AssociateRequestIndication, ProviderAbortIndication, PrimitiveError},
    table::TransitionTable,
};

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
    called_address: Option<String>,
    calling_address: Option<String>,
    table: TransitionTable,
}

impl UpperLayerConnection {
    pub fn new_no_assoc() -> Result<Self, UpperLayerStateMachineError> {
        Ok(Self {
            state: UpperLayerConnectionState::Idle,
            called_address: None,
            calling_address: None,
            table: TransitionTable::new()?,
        })
    }

    pub fn new_no_assoc_with_addresses(
        called_address: IpAddr, called_port: u16,
        calling_address: IpAddr, calling_port: u16,
    ) -> Result<Self, UpperLayerStateMachineError> {
        Ok(Self {
            state: UpperLayerConnectionState::Idle,
            called_address: Some(format_presentation_address(called_address, called_port)),
            calling_address: Some(format_presentation_address(calling_address, calling_port)),
            table: TransitionTable::new()?,
        })
    }

    pub fn new_client(
        called_address: IpAddr, called_port: u16,
        calling_address: IpAddr, calling_port: u16,
    ) -> Result<Self, UpperLayerStateMachineError> {
        Ok(Self {
            state: UpperLayerConnectionState::WaitingForOpenConnection,
            called_address: Some(format_presentation_address(called_address, called_port)),
            calling_address: Some(format_presentation_address(calling_address, calling_port)),
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
            CommandKind::ProviderAbortIndication => {
                Command::ProviderAbortIndication(ProviderAbortIndication::new(abort_reason))
            }

            CommandKind::AssociateIndication => {
                let Event::AssociateRequestPdu(pdu) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateIndication(AssociateRequestIndication::from_rq_pdu(
                    pdu,
                    conn.called_address.clone().unwrap(),
                    conn.calling_address.clone().unwrap(),
                ))
            }
            CommandKind::AssociateAcceptPdu => {
                let Event::AssociateResponsePrimitive(prim) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateAcceptPdu(AssociateRqAcPdu::try_from(prim)?)
            }
            CommandKind::AssociateRequestPdu => {
                let Event::ConnectionOpen(prim) = event.take().unwrap() else {
                    panic!()
                };

                Command::AssociateRequestPdu(AssociateRqAcPdu::try_from(prim)?)
            }
            CommandKind::AbortIndication => {
                let Event::AssociateAbortPdu(pdu) = event.take().unwrap() else {
                    panic!()
                };

                Command::AbortIndication(AbortIndication::from_pdu(pdu))
            }
            CommandKind::AssociateConfirmation => {
                match event.take().unwrap() {
                    Event::AssociateAcceptPdu(pdu) => {
                        Command::AssociateConfirmation(AssociateConfirmation::from_ac_pdu(pdu)?)
                    }
                    Event::AssociateRejectPdu(rj) => {
                        Command::AssociateConfirmation(AssociateConfirmation::from_rj_pdu(rj))
                    }
                    _ => panic!("unexpected event for AssociateConfirmation"),
                }
            }
        };
        out.push(cmd);
    }

    Ok((conn, out))
}

pub fn format_presentation_address(address: IpAddr, port: u16) -> String {
    format!("dicom:{address}:{port}")
}
