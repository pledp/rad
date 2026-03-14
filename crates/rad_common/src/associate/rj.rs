use crate::pdu::PduType;

pub enum RejectResult {
    Permanent,
    Transient
}

pub enum RejectSource {
    ServiceUser,
    Acse,
    Presentation
}

pub enum RejectReason {
    ServiceUser(ServiceUserReason),
    Acse(AcseReason),
    Presentation(PresentationReason)
}

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
    LocalLimitExceeded
}

struct AssociateRjPdu {
    pdu_type: PduType,
    length: u32,
    result: RejectResult,
    source: RejectSource,
    reason: Option<RejectReason>,
}
