use crate::pdu::PduType;

enum AbortSource {
    ServiceUser,
    ServiceProvider,
}

enum AbortReason {
    NoReason,
    UnexpectedPdu,
    UnrecognizedPduParam,
    UnexpectedPduParam,
    InvalidPduParam,
}

struct AssociateAbortPdu {
    pdu_type: PduType,
    length: u32,
    source: AbortSource,
    reason: Option<AbortReason>,
}
