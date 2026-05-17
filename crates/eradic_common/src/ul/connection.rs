use std::net::IpAddr;
use log::info;
use thiserror::Error;

use crate::ul::{associate::{AssociateRqAcPdu, AssociateRqAcPduError, PduDeserializationError, abort::{AbortReason, AbortSource, AssociateAbortPdu}}, event::{Command, Event}, service::{AbortIndication, AssociateRequestIndication, ProviderAbortIndication}};

#[derive(Debug, Error)]
pub enum UpperLayerConnectionError {
    #[error(transparent)]
    AssociateRqAcFromError(#[from] AssociateRqAcPduError)
}


#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UpperLayerConnectionState {
    /// Sta1
    Idle,
    /// Sta2
    WaitingForRequestPdu,
    // Sta 4
    WaitingForOpenConnection,
    // Sta 5
    WaitingForAcRjPdu,
    WaitingForResponsePrimitive,
    DataTransfer,

    /// DICOM UL State machine State 13 - Waiting for TCP connection to close.
    ///
    /// It is not required to transistion from State 13 to State 1 if the connection going out
    /// of scope implicitly indicates a transistion from State 13 to State 1.
    WaitForTcpClose,
}

/// DICOM upper layer Acceptor connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
#[derive(Clone, Debug)]
pub struct UpperLayerConnection {
    state: UpperLayerConnectionState,
    called_address: Option<String>,
    calling_address: Option<String>,
}

impl UpperLayerConnection {
    /// Creates a connection from Sta1. From Sta1 connection may become acceptor or requestor.
    pub fn new_no_assoc() -> Self {
        Self {
            state: UpperLayerConnectionState::Idle,
            called_address: None,
            calling_address: None,
        }
    }

    /// Creates a connection that starts from Sta5.
    pub fn new_client(
        called_address: IpAddr,
        called_port: u16,
        calling_address: IpAddr,
        calling_port: u16
    ) -> Self {
        Self {
            state: UpperLayerConnectionState::WaitingForAcRjPdu,
            called_address: Some(format_presentation_address(called_address, called_port)),
            calling_address: Some(format_presentation_address(calling_address, calling_port)),
        }
    }

    /// Creates a connection that starts from Sta2.
    pub fn new_server(
        called_address: IpAddr,
        called_port: u16,
        calling_address: IpAddr,
        calling_port: u16
    ) -> Self {
        Self {
            state: UpperLayerConnectionState::WaitingForRequestPdu,
            called_address: Some(format_presentation_address(called_address, called_port)),
            calling_address: Some(format_presentation_address(calling_address, calling_port)),
        }
    }

    /// DICOM state machine command dispatched given an [`Event`].
    ///
    /// See [DICOM standard part 8 chapter 9.2](https://dicom.nema.org/medical/dicom/current/output/html/part08.html#chapter_8)
    pub fn handle_event(
        &mut self,
        event: Event,
    ) -> Result<Vec<Command>, UpperLayerConnectionError> {
        let commands = match (event, self.state) {
            (Event::TransportConnectionIndication, _) => {
                self.state = UpperLayerConnectionState::WaitingForRequestPdu;
                vec![]
            }

           (Event::AssociateRequestPdu(pdu), _) => {
                self.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

                let indication = AssociateRequestIndication::from_rq_pdu(
                    pdu,
                    self.called_address.clone().unwrap(),
                    self.calling_address.clone().unwrap(),
                );

                vec![Command::AssociateIndication(indication)]
            }

            (Event::AssociateResponsePrimitiveAccept(prim), _) => {
                self.state = UpperLayerConnectionState::DataTransfer;

                vec![Command::AssociateAcceptPdu(AssociateRqAcPdu::try_from(prim)?)]
            }

            (Event::AssociateAbortPdu(pdu), _) => {
                self.state = UpperLayerConnectionState::Idle;

                vec![Command::AbortIndication(AbortIndication::from_pdu(pdu))]
            }

            (Event::TransportConnectionClosedIndication, state)
                if state != UpperLayerConnectionState::Idle =>
            {
                self.state = UpperLayerConnectionState::Idle;

                vec![Command::ProviderAbortIndication(ProviderAbortIndication::new(
                    AbortReason::NoReason
                ))]
            }

            (Event::AssociateAcceptPdu(_pdu), _) => {
                self.state = UpperLayerConnectionState::DataTransfer;

                info!("Accepted association");

                vec![]
            }

            (Event::UnrecognizedPdu, _) => {
                let commands = unrecognized_or_invalid_pdu(&self.state, AbortReason::UnrecognizedPdu);
                self.state = UpperLayerConnectionState::WaitForTcpClose;

                commands
            }

            _ => {
                let commands = unrecognized_or_invalid_pdu(&self.state, AbortReason::UnexpectedPdu);
                self.state = UpperLayerConnectionState::WaitForTcpClose;

                commands
            }
        };

        Ok(commands)
    }
}

fn unrecognized_or_invalid_pdu(state: &UpperLayerConnectionState, reason: AbortReason) -> Vec<Command> {
    let commands = match state {
        UpperLayerConnectionState::WaitingForRequestPdu => {
            vec![
                Command::AbortPdu(AssociateAbortPdu::new(
                    AbortSource::ServiceUser,
                    reason
                )),
                Command::StartArtimTimer
            ]
        }
        UpperLayerConnectionState::WaitForTcpClose => {
            vec![
                Command::AbortPdu(AssociateAbortPdu::new(
                    AbortSource::ServiceUser,
                    reason
                )),
            ]
        }
        _ => {
            vec![
                Command::AbortPdu(AssociateAbortPdu::new(
                    AbortSource::ServiceUser,
                    reason
                )),
                Command::StartArtimTimer,
                Command::ProviderAbortIndication(ProviderAbortIndication::new(
                    reason
                ))
            ]
        }
    };

    commands
}

pub fn format_presentation_address(called_address: IpAddr, called_port: u16) -> String {
    format!("dicom:{}:{}", called_address, called_port)
}
