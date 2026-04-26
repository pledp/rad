use crate::{
    associate::{AssociateRqAcPdu, abort::AssociateAbortPdu},
    service::{
        AcceptedAssociateRequestResponse, AssociateRequestIndication,
        RejectedAssociateRequestResponse,
    },
};

/// DICOM standard events
#[derive(Debug, PartialEq)]
pub enum Event {
    TransportConnectionIndication,
    ConnectionOpen,
    AssociateRequestPdu(AssociateRqAcPdu),
    DataPdu,
    AssociateRejectPdu,
    AssociateAcceptPdu,
    AssociateAbortPdu(AssociateAbortPdu),
    AssociateRequestPrimitive(AssociateRequestIndication),
    AssociateResponsePrimitiveReject(RejectedAssociateRequestResponse),
    AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse),
}

pub enum Command {
    AssociationIndication(AssociateRequestIndication),
    AbortIndication,
    AssociationResponse(RejectedAssociateRequestResponse),
    AssociateAcceptPdu(AcceptedAssociateRequestResponse),
    AssociateRequestPdu,
    OpenConnection,
}
