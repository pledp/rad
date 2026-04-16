use std::net::IpAddr;

use crate::{
    event::{Command, Event},
    service::AssociateRequestIndication,
};

use crate::Result;

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

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
pub struct UpperLayerConnection {
    state: UpperLayerConnectionState,
    called_address: Option<IpAddr>,
    calling_address: Option<IpAddr>,
}

impl UpperLayerConnection {
    pub fn new_client() -> Self {
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

    pub fn waiting_for_response_primitive(&mut self) {
        self.state = UpperLayerConnectionState::WaitingForRequestPdu;
    }
}
