use crate::{
    associate::AssociateRqAcPdu,
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
    AssociateRequestPrimitive(AssociateRequestIndication),
    AssociateResponsePrimitiveReject(RejectedAssociateRequestResponse),
    AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse),
}

pub enum Command {
    AssociationIndication(AssociateRequestIndication),
    AssociationResponse(RejectedAssociateRequestResponse),
    AssociateAcceptPdu(AcceptedAssociateRequestResponse),
    AssociateRequestPdu,
    OpenConnection,
}
