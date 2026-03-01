use rad_common::associate::AssociateRqAcPdu;

pub enum AssociationResult {
    Accepted,
    Rejected,
}

pub trait ApplicationEntity: Send + Sync {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult;
}
