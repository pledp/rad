use std::io::Read;

use thiserror::Error;

use crate::pdu::{PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding};
use crate::ul::associate::{AssociationResult, AssociationResultError, PduDeserializationError};

#[derive(Debug, Error)]
pub enum RejectParseError {
    #[error(transparent)]
    InvalidResult(#[from] AssociationResultError),
    #[error("invalid reject source: {0}")]
    InvalidSource(u8),
    #[error("invalid reject reason for source {src:?}: {reason}")]
    InvalidReason { src: RejectSource, reason: u8 },
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RejectSource {
    ServiceUser = 0x01,
    Acse = 0x02,
    Presentation = 0x03,
}

impl TryFrom<u8> for RejectSource {
    type Error = RejectParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(RejectSource::ServiceUser),
            0x02 => Ok(RejectSource::Acse),
            0x03 => Ok(RejectSource::Presentation),
            v => Err(RejectParseError::InvalidSource(v)),
        }
    }
}

impl From<RejectSource> for u8 {
    fn from(value: RejectSource) -> Self {
        match value {
            RejectSource::ServiceUser => 0x01,
            RejectSource::Acse => 0x02,
            RejectSource::Presentation => 0x03,
        }
    }
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
///
/// # Errors
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/reject_errors.md"))]
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
    let result: AssociationResult = result_buf[0]
        .try_into()
        .map_err(|e: AssociationResultError| PduDeserializationError::InvalidRejectPdu(e.into()))?;

    let mut source_buf = [0u8; 1];
    reader.read_exact(&mut source_buf)?;
    let source: RejectSource = source_buf[0]
        .try_into()
        .map_err(PduDeserializationError::InvalidRejectPdu)?;

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
        (source, v) => return Err(PduDeserializationError::InvalidRejectPdu(
            RejectParseError::InvalidReason { src: *source, reason: v },
        )),
    };

    Ok(AssociateRjPdu { result, source, reason })
}
#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    fn rj_bytes(result: u8, source: u8, reason: u8) -> Vec<u8> {
        vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, result, source, reason]
    }

    #[test]
    fn test_deserialize_reject_pdu_ok() {
        let mut reader = Cursor::new(rj_bytes(1, 1, 1));
        let pdu = deserialize_reject_pdu(&mut reader).unwrap();
        assert_eq!(pdu.result, AssociationResult::RejectedPermanent);
        assert_eq!(pdu.source, RejectSource::ServiceUser);
        assert_eq!(pdu.reason, Some(RejectReason::ServiceUser(ServiceUserReason::NoReason)));
    }

    #[test]
    fn test_deserialize_reject_pdu_rejected_transient() {
        let mut reader = Cursor::new(rj_bytes(2, 1, 1));
        let pdu = deserialize_reject_pdu(&mut reader).unwrap();
        assert_eq!(pdu.result, AssociationResult::RejectedTransient);
    }

    #[test]
    fn test_deserialize_reject_pdu_service_user_reasons() {
        let cases = [
            (1, RejectReason::ServiceUser(ServiceUserReason::NoReason)),
            (2, RejectReason::ServiceUser(ServiceUserReason::ApplicationContextNameNotSupported)),
            (3, RejectReason::ServiceUser(ServiceUserReason::CallingAeNotRecognized)),
            (7, RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized)),
        ];
        for (reason_byte, expected) in cases {
            let mut reader = Cursor::new(rj_bytes(1, 1, reason_byte));
            let pdu = deserialize_reject_pdu(&mut reader).unwrap();
            assert_eq!(pdu.source, RejectSource::ServiceUser);
            assert_eq!(pdu.reason, Some(expected));
        }
    }

    #[test]
    fn test_deserialize_reject_pdu_acse_reasons() {
        let cases = [
            (1, RejectReason::Acse(AcseReason::NoReason)),
            (2, RejectReason::Acse(AcseReason::ProtocolNotSupported)),
        ];
        for (reason_byte, expected) in cases {
            let mut reader = Cursor::new(rj_bytes(1, 2, reason_byte));
            let pdu = deserialize_reject_pdu(&mut reader).unwrap();
            assert_eq!(pdu.source, RejectSource::Acse);
            assert_eq!(pdu.reason, Some(expected));
        }
    }

    #[test]
    fn test_deserialize_reject_pdu_presentation_reasons() {
        let cases = [
            (1, RejectReason::Presentation(PresentationReason::TemporaryCongestion)),
            (2, RejectReason::Presentation(PresentationReason::LocalLimitExceeded)),
        ];
        for (reason_byte, expected) in cases {
            let mut reader = Cursor::new(rj_bytes(1, 3, reason_byte));
            let pdu = deserialize_reject_pdu(&mut reader).unwrap();
            assert_eq!(pdu.source, RejectSource::Presentation);
            assert_eq!(pdu.reason, Some(expected));
        }
    }

    #[test]
    fn test_deserialize_reject_pdu_invalid_result() {
        let mut reader = Cursor::new(rj_bytes(3, 1, 1));
        assert!(matches!(
            deserialize_reject_pdu(&mut reader),
            Err(PduDeserializationError::InvalidRejectPdu(RejectParseError::InvalidResult(
                AssociationResultError::InvalidValue(3)
            )))
        ));
    }

    #[test]
    fn test_deserialize_reject_pdu_invalid_source() {
        let mut reader = Cursor::new(rj_bytes(1, 0, 1));
        assert!(matches!(
            deserialize_reject_pdu(&mut reader),
            Err(PduDeserializationError::InvalidRejectPdu(RejectParseError::InvalidSource(0)))
        ));
    }

    #[test]
    fn test_deserialize_reject_pdu_invalid_reason() {
        for (source, bad_reason) in [(1u8, 5u8), (2, 3), (3, 3)] {
            let mut reader = Cursor::new(rj_bytes(1, source, bad_reason));
            assert!(matches!(
                deserialize_reject_pdu(&mut reader),
                Err(PduDeserializationError::InvalidRejectPdu(RejectParseError::InvalidReason { .. }))
            ));
        }
    }

    #[test]
    fn test_deserialize_reject_pdu_wrong_pdu_type() {
        let mut data = rj_bytes(1, 1, 1);
        data[0] = 0x01;
        let mut reader = Cursor::new(data);
        assert!(matches!(
            deserialize_reject_pdu(&mut reader),
            Err(PduDeserializationError::UnexpectedPduType(PduType::AssociateRequest))
        ));
    }

    #[test]
    fn test_deserialize_reject_pdu_invalid_length() {
        // Empty
        assert!(matches!(
            deserialize_reject_pdu(&mut Cursor::new(vec![])),
            Err(PduDeserializationError::InvalidLength(_))
        ));
        // Missing reason byte
        assert!(matches!(
            deserialize_reject_pdu(&mut Cursor::new(vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x01])),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }
}
