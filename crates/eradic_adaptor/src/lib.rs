use rad_common::associate::AssociateRqAcPdu;
use rad_common::associate::rj::{RejectSource, RejectReason, RejectResult};

use std::collections::HashMap;
use std::string::String;

pub enum AssociationResult {
    Accepted,
    Rejected { result: RejectResult, source: RejectSource, reason: RejectReason },
    Abort,
}

pub trait ApplicationEntityAdapter: Send + Sync {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult;
}

pub type ApplicationEntityRegistry = HashMap<String, Box<dyn ApplicationEntityAdapter>>;

pub fn handle_associate_request(registry: &mut ApplicationEntityRegistry) -> AssociationResult {
    /*
    let result = match registry.get(&pdu.called_ae) {
        Some(ae) => {
            ae.handle_associate_request(pdu)
        }
        None => {
            todo!()
        }
    }
    */

    todo!()
}
