use crate::{associate::RejectedAssociationResult, pdu::PduType};

pub enum RejectSource {
    ServiceUser,
    Acse,
    Presentation,
}

pub enum RejectReason {
    ServiceUser(ServiceUserReason),
    Acse(AcseReason),
    Presentation(PresentationReason),
}

#[derive(Debug)]
pub enum ServiceUserReason {
    NoReason,
    ApplicationContextNameNotSupported,
    CalingAeNotRecognized,
    CalledAeNotRecognized,
}

pub enum AcseReason {
    NoReason,
    ProtocolNotSupported,
}

pub enum PresentationReason {
    TemporaryCongestion,
    LocalLimitExceeded,
}

struct AssociateRjPdu {
    pdu_type: PduType,
    length: u32,
    result: RejectedAssociationResult,
    source: RejectSource,
    reason: Option<RejectReason>,
}
