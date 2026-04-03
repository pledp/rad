use crate::{associate::AssociateRqAcPdu, service::{AcceptedAssociateRequestResponse, AssociateRequestIndication, AssociateRequestResponse, RejectedAssociateRequestResponse}};

pub enum Event {
    AssociateRequestPdu(AssociateRqAcPdu),
    DataPdu,
    AssociateRejectPdu,
    AssociateAcceptPdu,
    AssociateResponsePrimitiveReject(RejectedAssociateRequestResponse),
    AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse),
}

pub enum Command {
    AssociationIndication(AssociateRequestIndication),
    AssociationResponse(RejectedAssociateRequestResponse),
    AssociateAcceptPdu(AcceptedAssociateRequestResponse)
}
