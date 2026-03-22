mod rq_ac;
mod abort;
pub mod rj;
mod user_information;

pub use rq_ac::*;
pub use abort::*;
pub use user_information::*;

use rj::{RejectReason, RejectResult, RejectSource, AcseReason};
use crate::Result;

fn service_provider_rq_pdu_validation(pdu: &AssociateRqAcPdu) -> Result<Option<AssociationResult>> {
    let source = RejectSource::Acse;

    if pdu.protocol_version != 1 {
        return Ok(Some(AssociationResult::Rejected {
            result: RejectResult::Transient,
            source,
            reason: RejectReason::Acse(AcseReason::ProtocolNotSupported)
        }))
    }

    todo!();
}

pub enum AssociationResult {
    Accepted,
    Rejected { result: RejectResult, source: RejectSource, reason: RejectReason },
    Abort,
}
