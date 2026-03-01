use rad_common::associate::AssociateRqAcPdu;

use crate::adapter::{ApplicationEntity, AssociationResult};

pub struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult {
        AssociationResult::Accepted
    }
}
