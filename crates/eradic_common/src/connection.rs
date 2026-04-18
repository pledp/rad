use std::net::IpAddr;

use crate::{
    event::{Command, Event},
    service::AssociateRequestIndication,
};

use crate::Result;

#[derive(Clone, Copy)]
pub enum UpperLayerConnectionState {
    /// Sta1
    NoAssociation,
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
#[derive(Clone, Copy)]
pub struct UpperLayerAcceptorConnection {
    state: UpperLayerConnectionState,
    called_address: Option<IpAddr>,
    calling_address: Option<IpAddr>,
}

// TODO: Remove new_client, create UpperLayerRequestorConnection
impl UpperLayerAcceptorConnection {
    pub fn new_client() -> Self {
        Self::new_no_assoc()
    }

    pub fn new_no_assoc() -> Self {
        Self {
            state: UpperLayerConnectionState::NoAssociation,
            called_address: None,
            calling_address: None,
        }
    }

    pub fn new_server(called_address: IpAddr, calling_address: IpAddr) -> Self {
        Self {
            state: UpperLayerConnectionState::WaitingForRequestPdu,
            called_address: Some(called_address),
            calling_address: Some(calling_address),
        }
    }

    pub fn waiting_for_response_primitive(&mut self) {
        self.state = UpperLayerConnectionState::WaitingForRequestPdu;
    }

    pub fn handle_event(&mut self, event: Event) -> Result<Option<Command>> {
        let command = match event {
            Event::TransportConnectionIndication => {
                self.waiting_for_response_primitive();
                None
            }

            Event::AssociateRequestPdu(pdu) => {
                self.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

                let indication = AssociateRequestIndication::from_rq_pdu(
                    pdu,
                    &self.called_address.unwrap(),
                    &self.calling_address.unwrap(),
                );

                Some(Command::AssociationIndication(indication))
            }
            Event::AssociateResponsePrimitiveAccept(prim) => {
                self.state = UpperLayerConnectionState::DataTransfer;

                Some(Command::AssociateAcceptPdu(prim))
            }
            Event::AssociateRequestPrimitive(indication) => {
                self.state = UpperLayerConnectionState::WaitingForOpenConnection;

                Some(Command::OpenConnection)
            }
            Event::ConnectionOpen => {
                self.state = UpperLayerConnectionState::WaitingForAcRjPdu;

                Some(Command::AssociateRequestPdu)
            }

            _ => {
                todo!()
            }
        };

        Ok(command)
    }
}

pub fn handle_server_event(conn: &UpperLayerAcceptorConnection, event: Event) -> Result<(Option<Command>, UpperLayerAcceptorConnection)> {
    let mut new_state = conn.clone();

    let command = match event {
        Event::TransportConnectionIndication => {
            new_state.waiting_for_response_primitive();
            None
        }

        Event::AssociateRequestPdu(pdu) => {
            new_state.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

            let indication = AssociateRequestIndication::from_rq_pdu(
                pdu,
                &new_state.called_address.unwrap(),
                &new_state.calling_address.unwrap(),
            );

            Some(Command::AssociationIndication(indication))
        }
        Event::AssociateResponsePrimitiveAccept(prim) => {
            new_state.state = UpperLayerConnectionState::DataTransfer;

            Some(Command::AssociateAcceptPdu(prim))
        }
        Event::AssociateRequestPrimitive(indication) => {
            new_state.state = UpperLayerConnectionState::WaitingForOpenConnection;

            Some(Command::OpenConnection)
        }
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
    state: UpperLayerConnectionState
}

impl UpperLayerRequestorConnection {
    pub fn new_client() -> Self {
        Self::new_no_assoc()
    }

    pub fn new_no_assoc() -> Self {
        Self {
            state: UpperLayerConnectionState::NoAssociation,
        }
    }

    pub fn handle_event(&mut self, event: Event) -> Result<Option<Command>> {
        let command = match event {
            Event::ConnectionOpen => {
                self.state = UpperLayerConnectionState::WaitingForAcRjPdu;

                Some(Command::AssociateRequestPdu)
            }
            _ => todo!()
        };

        Ok(command)
    }
}
