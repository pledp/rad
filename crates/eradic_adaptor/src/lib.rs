use async_trait::async_trait;

use rad_common::associate::AssociateRqAcPdu;
use rad_common::associate::rj::{RejectSource, RejectReason, RejectResult};

pub enum AssociationResult {
    Accepted,
    Rejected { result: RejectResult, source: RejectSource, reason: RejectReason },
    Abort,
}

#[async_trait]
pub trait UpperLayerServiceUser: Send + Sync {
    async fn handle_associate_request(&mut self, pdu: AssociateRqAcPdu) -> AssociationResult;
}
