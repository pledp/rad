use std::net::IpAddr;
use log::info;
use thiserror::Error;

use crate::ul::{associate::{AssociateRqAcPdu, AssociateRqAcPduError, PduDeserializationError, abort::{AbortReason, AbortSource}}, event::{Command, Event}, service::{AbortIndication, AssociateRequestIndication, ProviderAbortIndication}};

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

    /// Creates a connection that starts from Sta4.
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
    ) -> Result<Option<Command>, UpperLayerConnectionError> {
        let command = match (event, self.state) {
            (Event::TransportConnectionIndication, _) => {
                self.state = UpperLayerConnectionState::WaitingForRequestPdu;
                None
            }

           (Event::AssociateRequestPdu(pdu), _) => {
                self.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

                let indication = AssociateRequestIndication::from_rq_pdu(
                    pdu,
                    self.called_address.clone().unwrap(),
                    self.calling_address.clone().unwrap(),
                );

                Some(Command::AssociateIndication(indication))
            }

            (Event::AssociateResponsePrimitiveAccept(prim), _) => {
                self.state = UpperLayerConnectionState::DataTransfer;


                Some(Command::AssociateAcceptPdu(AssociateRqAcPdu::try_from(prim)?))
            }

            (Event::AssociateAbortPdu(pdu), _) => {
                self.state = UpperLayerConnectionState::Idle;

                Some(Command::AbortIndication(AbortIndication::from_pdu(pdu)))
            }

            (Event::TransportConnectionClosedIndication, state)
                if state != UpperLayerConnectionState::Idle =>
            {
                self.state = UpperLayerConnectionState::Idle;

                Some(Command::ProviderAbortIndication(ProviderAbortIndication::new(
                    AbortReason::NoReason
                )))
            }

            (Event::AssociateAcceptPdu(pdu), _) => {
                self.state = UpperLayerConnectionState::DataTransfer;

                info!("Accepted association");

                None
            }

            _ => {
                todo!()
            }
        };

        Ok(command)
    }
}

pub fn format_presentation_address(called_address: IpAddr, called_port: u16) -> String {
    format!("dicom:{}:{}", called_address, called_port)
}
