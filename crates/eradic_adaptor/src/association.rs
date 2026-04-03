use std::net::IpAddr;

use rad_common::{
    service::{AssociateRequestIndication},
    event::{Event, Command},
};

use crate::{Result};

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
pub struct UpperLayerConnection {
    state: UpperLayerConnectionState,
    called_address: IpAddr,
    calling_address: IpAddr,
}

impl UpperLayerConnection {
    pub fn new(called_address: IpAddr, calling_address: IpAddr) -> Self {
        Self {
            state: UpperLayerConnectionState::WaitingForRequestPdu,
            called_address,
            calling_address
        }
    }

    pub fn handle_event(
        &mut self,
        event: Event
    ) -> Result<Option<Command>>{
        match event {
            Event::AssociateRequestPdu(pdu) => {
                self.state = UpperLayerConnectionState::WaitingForResponsePrimitive;

                let indication = AssociateRequestIndication::from_rq_pdu(
                    pdu, &self.called_address, &self.calling_address,
                );

                Ok(Some(
                    Command::AssociationIndication(indication)
                ))
            }
            Event::AssociateResponsePrimitiveAccept(prim) => {
                self.state = UpperLayerConnectionState::DataTransfer;

                Ok(Some(
                    Command::AssociateAcceptPdu(prim)
                ))
            }
            _ => {
                todo!()
            }
        }
    }
}

pub enum UpperLayerConnectionState {
    WaitingForRequestPdu,
    WaitingForResponsePrimitive,
    DataTransfer,
    WaitForTcpClose
}
