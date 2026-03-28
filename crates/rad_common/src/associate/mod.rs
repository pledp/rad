mod abort;
pub mod rj;
mod rq_ac;
mod user_information;

pub use abort::*;
pub use rq_ac::*;
pub use user_information::*;

use crate::Result;
use rj::{AcseReason, RejectReason, RejectResult, RejectSource};

fn service_provider_rq_pdu_validation(pdu: &AssociateRqAcPdu) -> Result<Option<AssociationResult>> {
    let source = RejectSource::Acse;

    if pdu.protocol_version != 1 {
        return Ok(Some(AssociationResult::Rejected {
            result: RejectResult::Transient,
            source,
            reason: RejectReason::Acse(AcseReason::ProtocolNotSupported),
        }));
    }

    todo!();
}

pub enum AssociationResult {
    Accepted,
    Rejected {
        result: RejectResult,
        source: RejectSource,
        reason: RejectReason,
    },
    Abort,
}
