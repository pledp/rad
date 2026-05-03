use std::net::IpAddr;

use crate::{
    associate::abort::{AbortReason, AbortSource}, ul::event::{Command, Event}, ul::service::{AbortIndication, AssociateRequestIndication, ProviderAbortIndication}
};

use crate::Result;

#[derive(Clone, Copy, PartialEq)]
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
#[derive(Clone)]
pub struct UpperLayerAcceptorConnection {
    state: UpperLayerConnectionState,
    called_address: Option<String>,
    calling_address: Option<String>,
}

// TODO: Remove new_client, create UpperLayerRequestorConnection, remove Option<>
impl UpperLayerAcceptorConnection {
    pub fn new_client() -> Self {
        Self::new_no_assoc()
    }

    pub fn new_no_assoc() -> Self {
        Self {
            state: UpperLayerConnectionState::Idle,
            called_address: None,
            calling_address: None,
        }
    }

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
}

/// DICOM state machine command dispatched given an [`Event`].
///
/// See [DICOM standard part 8 chapter 9.2](https://dicom.nema.org/medical/dicom/current/output/html/part08.html#chapter_8)
pub fn handle_server_event(
    conn: UpperLayerAcceptorConnection,
    event: Event,
) -> Result<(Option<Command>, UpperLayerAcceptorConnection)> {
    let mut new_state = conn;

    let command = match (event, new_state.state) {
        (Event::TransportConnectionIndication, _) => {
            new_state.state = UpperLayerConnectionState::WaitingForRequestPdu;
            None
        }

       (Event::AssociateRequestPdu(pdu), _) => {
            new_state.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

            let indication = AssociateRequestIndication::from_rq_pdu(
                pdu,
                new_state.called_address.clone().unwrap(),
                new_state.calling_address.clone().unwrap(),
            );

            Some(Command::AssociateIndication(indication))
        }

        (Event::AssociateResponsePrimitiveAccept(prim), _) => {
            new_state.state = UpperLayerConnectionState::DataTransfer;

            Some(Command::AssociateAcceptPdu(prim))
        }

        (Event::AssociateAbortPdu(pdu), _) => {
            new_state.state = UpperLayerConnectionState::Idle;

            Some(Command::AbortIndication(AbortIndication::from_pdu(pdu)))
        }

        (Event::TransportConnectionClosedIndication, state)
            if state != UpperLayerConnectionState::Idle =>
        {
            new_state.state = UpperLayerConnectionState::Idle;

            Some(Command::ProviderAbortIndication(ProviderAbortIndication::new(
                AbortReason::NoReason
            )))
        }

        _ => {
            None
        }
    };

    Ok((command, new_state))
}

pub fn handle_client_event(
    conn: UpperLayerRequestorConnection,
    event: Event,
) -> Result<(Option<Command>, UpperLayerRequestorConnection)> {
    let mut new_state = conn;

    let command = match event {
        Event::ConnectionOpen => {
            new_state.state = UpperLayerConnectionState::WaitingForAcRjPdu;

            Some(Command::AssociateRequestPdu)
        }

        _ => {
            todo!()
        }
    };

    Ok((command, new_state))
}

/// DICOM upper layer Acceptor connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
pub struct UpperLayerRequestorConnection {
    state: UpperLayerConnectionState,
}

impl UpperLayerRequestorConnection {
    pub fn new_client() -> Self {
        Self {
            state: UpperLayerConnectionState::WaitingForOpenConnection,
        }
    }

}

pub fn format_presentation_address(called_address: IpAddr, called_port: u16) -> String {
    format!("dicom:{}:{}", called_address, called_port)
}
