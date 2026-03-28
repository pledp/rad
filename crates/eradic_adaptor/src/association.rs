use std::net::IpAddr;

use rad_common::{
    associate::{AssociateRqAcPdu, AssociationResult},
    pdu::{PDU_HEADER_LENGTH, PduHeader, PduType, read_pdu_header},
    service::AssociateRequestIndication,
};

use crate::{Error, Result, issue_indication_from_pdu};

/// DICOM upper layer connection.
/// The DICOM standard defines different states for the system. Different states transition differently
/// depending on performed actions.
///
/// See [DICOM standard part 8](https://dicom.nema.org/medical/dicom/current/output/html/part08).
pub enum UpperLayerConnection {
    WaitingForRequestPdu(WaitingForRequestPdu),
    WaitingForResponsePrimitive(WaitingForResponsePrimitive),
}

impl UpperLayerConnection {
    pub fn new() -> Self {
        UpperLayerConnection::WaitingForRequestPdu(WaitingForRequestPdu::new())
    }
}

/// DICOM upper layer connection state 2 (Sta2).
///
/// Waiting for A-ASSOCIATE-RQ PDU from client.
pub struct WaitingForRequestPdu {}

impl WaitingForRequestPdu {
    pub fn new() -> Self {
        Self {}
    }

    /// Incoming A-ASSOCIATE-RQ PDU
    pub fn association_request(
        self,
        pdu: AssociateRqAcPdu,
        called: IpAddr,
        calling: IpAddr,
    ) -> Result<(WaitingForResponsePrimitive, AssociateRequestIndication)> {
        let indication = issue_indication_from_pdu(pdu, called, calling)?;

        Ok((WaitingForResponsePrimitive::from_waiting(self), indication))
    }
}

pub struct WaitingForResponsePrimitive {}

impl WaitingForResponsePrimitive {
    fn from_waiting(prev_state: WaitingForRequestPdu) -> Self {
        Self {}
    }

    pub fn handle_response_primitive(self) -> ResponsePrimitiveResultState {
        ResponsePrimitiveResultState::DataTransfer(DataTransfer {})
    }
}

pub enum ResponsePrimitiveResultState {
    DataTransfer(DataTransfer),
}

pub struct DataTransfer {}
