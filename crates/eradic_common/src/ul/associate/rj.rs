use std::io::Read;

use thiserror::Error;

use crate::pdu::{PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding};
use crate::ul::associate::{AssociationResult, PduDeserializationError};

#[derive(Debug, Error)]
pub enum RejectParseError {
    #[error("invalid reject result: {0}")]
    InvalidResult(u8),
    #[error("invalid reject source: {0}")]
    InvalidSource(u8),
    #[error("invalid reject reason for source {src}: {reason}")]
    InvalidReason { src: u8, reason: u8 },
}

#[derive(Debug, PartialEq)]
pub enum RejectSource {
    ServiceUser,
    Acse,
    Presentation,
}

#[derive(Debug, PartialEq)]
pub enum RejectReason {
    ServiceUser(ServiceUserReason),
    Acse(AcseReason),
    Presentation(PresentationReason),
}

#[derive(Debug, PartialEq)]
pub enum ServiceUserReason {
    NoReason,
    ApplicationContextNameNotSupported,
    CallingAeNotRecognized,
    CalledAeNotRecognized,
}

#[derive(Debug, PartialEq)]
pub enum AcseReason {
    NoReason,
    ProtocolNotSupported,
}

#[derive(Debug, PartialEq)]
pub enum PresentationReason {
    TemporaryCongestion,
    LocalLimitExceeded,
}

#[derive(Debug, PartialEq)]
pub struct AssociateRjPdu {
    pub result: AssociationResult,
    pub source: RejectSource,
    pub reason: Option<RejectReason>,
}

/// Deserializes bytes from a [Read] into an [AssociateRjPdu].
/// The reader must be positioned at the start of the PDU (PDU-type byte).
pub fn deserialize_reject_pdu<R: Read>(
    reader: &mut R,
) -> Result<AssociateRjPdu, PduDeserializationError> {
    let mut type_buf = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut type_buf)?;

    let pdu_type = PduType::try_from(type_buf[0])?;
    if pdu_type != PduType::AssociateReject {
        return Err(PduDeserializationError::UnexpectedPduType(pdu_type));
    }

    read_padding(reader, 1);

    let mut length_buf = [0u8; PDU_LENGTH_LENGTH];
    reader.read_exact(&mut length_buf)?;

    read_padding(reader, 1);

    let mut result_buf = [0u8; 1];
    reader.read_exact(&mut result_buf)?;
    let result = match result_buf[0] {
        1 => AssociationResult::RejectedPermanent,
        2 => AssociationResult::RejectedTransient,
        v => return Err(PduDeserializationError::InvalidRejectPdu(RejectParseError::InvalidResult(v))),
    };

    let mut source_buf = [0u8; 1];
    reader.read_exact(&mut source_buf)?;
    let source = match source_buf[0] {
        1 => RejectSource::ServiceUser,
        2 => RejectSource::Acse,
        3 => RejectSource::Presentation,
        v => return Err(PduDeserializationError::InvalidRejectPdu(RejectParseError::InvalidSource(v))),
    };

    let mut reason_buf = [0u8; 1];
    reader.read_exact(&mut reason_buf)?;
    let reason = match (&source, reason_buf[0]) {
        (RejectSource::ServiceUser, 1) => Some(RejectReason::ServiceUser(ServiceUserReason::NoReason)),
        (RejectSource::ServiceUser, 2) => Some(RejectReason::ServiceUser(ServiceUserReason::ApplicationContextNameNotSupported)),
        (RejectSource::ServiceUser, 3) => Some(RejectReason::ServiceUser(ServiceUserReason::CallingAeNotRecognized)),
        (RejectSource::ServiceUser, 7) => Some(RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized)),
        (RejectSource::Acse, 1) => Some(RejectReason::Acse(AcseReason::NoReason)),
        (RejectSource::Acse, 2) => Some(RejectReason::Acse(AcseReason::ProtocolNotSupported)),
        (RejectSource::Presentation, 1) => Some(RejectReason::Presentation(PresentationReason::TemporaryCongestion)),
        (RejectSource::Presentation, 2) => Some(RejectReason::Presentation(PresentationReason::LocalLimitExceeded)),
        (_, v) => return Err(PduDeserializationError::InvalidRejectPdu(
            RejectParseError::InvalidReason { src: source_buf[0], reason: v },
        )),
    };

    Ok(AssociateRjPdu { result, source, reason })
}
