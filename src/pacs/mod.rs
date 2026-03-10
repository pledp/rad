use rad_common::associate::AssociateRqAcPdu;

use eradic_adaptor::{ApplicationEntityAdapter, AssociationResult};

pub struct Pacs {}

impl ApplicationEntityAdapter for Pacs {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult {
        AssociationResult::Accepted
    }
}
