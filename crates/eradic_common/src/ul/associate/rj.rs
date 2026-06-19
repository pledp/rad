use std::io::Read;

use thiserror::Error;

use crate::pdu::{PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding, vec8_add_padding};
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RejectReason {
    ServiceUser(ServiceUserReason),
    Acse(AcseReason),
    Presentation(PresentationReason),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ServiceUserReason {
    NoReason,
    ApplicationContextNameNotSupported,
    CallingAeNotRecognized,
    CalledAeNotRecognized,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AcseReason {
    NoReason,
    ProtocolNotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PresentationReason {
    TemporaryCongestion,
    LocalLimitExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssociateRjPdu {
    pub result: AssociationResult,
    pub source: RejectSource,
    pub reason: RejectReason,
}

/// Serializes an [AssociateRjPdu] into a [`Vec<u8>`].
///
/// See [DICOM standard part 8, §9.3.4](https://dicom.nema.org/medical/dicom/current/output/html/part08.html#sect_9.3.4)
pub fn serialize_reject_pdu(item: &AssociateRjPdu) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(PduType::AssociateReject.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&4u32.to_be_bytes());

    vec8_add_padding(&mut pdu, 1);
    pdu.push(item.result.into());
    pdu.push(item.source.into());
    pdu.push(reject_reason_to_byte(item.reason));

    pdu
}

/// Maps a [RejectReason] to its single-byte wire encoding. The valid byte values are
/// source-specific, per the reason tables in DICOM PS3.8 §9.3.4.
fn reject_reason_to_byte(reason: RejectReason) -> u8 {
    match reason {
        RejectReason::ServiceUser(ServiceUserReason::NoReason) => 1,
        RejectReason::ServiceUser(ServiceUserReason::ApplicationContextNameNotSupported) => 2,
        RejectReason::ServiceUser(ServiceUserReason::CallingAeNotRecognized) => 3,
        RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized) => 7,
        RejectReason::Acse(AcseReason::NoReason) => 1,
        RejectReason::Acse(AcseReason::ProtocolNotSupported) => 2,
        RejectReason::Presentation(PresentationReason::TemporaryCongestion) => 1,
        RejectReason::Presentation(PresentationReason::LocalLimitExceeded) => 2,
    }
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
        (RejectSource::ServiceUser, 1) => RejectReason::ServiceUser(ServiceUserReason::NoReason),
        (RejectSource::ServiceUser, 2) => RejectReason::ServiceUser(ServiceUserReason::ApplicationContextNameNotSupported),
        (RejectSource::ServiceUser, 3) => RejectReason::ServiceUser(ServiceUserReason::CallingAeNotRecognized),
        (RejectSource::ServiceUser, 7) => RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized),
        (RejectSource::Acse, 1) => RejectReason::Acse(AcseReason::NoReason),
        (RejectSource::Acse, 2) => RejectReason::Acse(AcseReason::ProtocolNotSupported),
        (RejectSource::Presentation, 1) => RejectReason::Presentation(PresentationReason::TemporaryCongestion),
        (RejectSource::Presentation, 2) => RejectReason::Presentation(PresentationReason::LocalLimitExceeded),
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
        assert_eq!(pdu.reason, RejectReason::ServiceUser(ServiceUserReason::NoReason));
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
            assert_eq!(pdu.reason, expected);
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
            assert_eq!(pdu.reason, expected);
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
            assert_eq!(pdu.reason, expected);
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
    fn test_reject_source_from_u8_encodes_all_variants_correctly() {
        let cases = [
            (RejectSource::ServiceUser, 0x01u8),
            (RejectSource::Acse, 0x02),
            (RejectSource::Presentation, 0x03),
        ];
        for (variant, expected_byte) in cases {
            assert_eq!(u8::from(variant), expected_byte);
        }
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

    // --- serialize_reject_pdu tests ---

    #[test]
    fn test_serialize_reject_pdu_produces_correct_bytes() {
        let pdu = AssociateRjPdu {
            result: AssociationResult::RejectedPermanent,
            source: RejectSource::ServiceUser,
            reason: RejectReason::ServiceUser(ServiceUserReason::NoReason),
        };

        // DICOM PS3.8 §9.3.4: 03H, reserved, length=4 (4 bytes BE), reserved, result, source, reason
        assert_eq!(
            serialize_reject_pdu(&pdu),
            vec![0x03, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01, 0x01, 0x01]
        );
    }

    #[test]
    fn test_serialize_reject_pdu_output_is_always_10_bytes() {
        let pdu = AssociateRjPdu {
            result: AssociationResult::RejectedTransient,
            source: RejectSource::Presentation,
            reason: RejectReason::Presentation(PresentationReason::LocalLimitExceeded),
        };
        assert_eq!(serialize_reject_pdu(&pdu).len(), 10);
    }

    #[test]
    fn test_serialize_reject_pdu_first_byte_is_reject_pdu_type() {
        let pdu = AssociateRjPdu {
            result: AssociationResult::RejectedPermanent,
            source: RejectSource::Acse,
            reason: RejectReason::Acse(AcseReason::NoReason),
        };
        assert_eq!(serialize_reject_pdu(&pdu)[0], 0x03);
    }

    #[test]
    fn test_serialize_reject_pdu_reserved_padding_bytes_are_zero() {
        let pdu = AssociateRjPdu {
            result: AssociationResult::RejectedPermanent,
            source: RejectSource::ServiceUser,
            reason: RejectReason::ServiceUser(ServiceUserReason::NoReason),
        };
        let bytes = serialize_reject_pdu(&pdu);
        assert_eq!(bytes[1], 0x00, "byte 1 must be reserved 0x00");
        assert_eq!(bytes[6], 0x00, "byte 6 must be reserved 0x00");
    }

    #[test]
    fn test_serialize_reject_pdu_length_field_is_4_big_endian() {
        let pdu = AssociateRjPdu {
            result: AssociationResult::RejectedPermanent,
            source: RejectSource::ServiceUser,
            reason: RejectReason::ServiceUser(ServiceUserReason::NoReason),
        };
        let bytes = serialize_reject_pdu(&pdu);
        let length = u32::from_be_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        assert_eq!(length, 4);
    }

    #[test]
    fn test_serialize_reject_pdu_encodes_all_sources_correctly() {
        let cases: &[(RejectSource, u8)] = &[
            (RejectSource::ServiceUser, 0x01),
            (RejectSource::Acse, 0x02),
            (RejectSource::Presentation, 0x03),
        ];
        for &(source, expected) in cases {
            let pdu = AssociateRjPdu {
                result: AssociationResult::RejectedPermanent,
                source,
                reason: RejectReason::ServiceUser(ServiceUserReason::NoReason),
            };
            assert_eq!(serialize_reject_pdu(&pdu)[8], expected, "source {:?} should encode as {:#04x}", source, expected);
        }
    }

    #[test]
    fn test_serialize_reject_pdu_encodes_all_results_correctly() {
        let cases: &[(AssociationResult, u8)] = &[
            (AssociationResult::Accepted, 0x00),
            (AssociationResult::RejectedPermanent, 0x01),
            (AssociationResult::RejectedTransient, 0x02),
        ];
        for &(result, expected) in cases {
            let pdu = AssociateRjPdu {
                result,
                source: RejectSource::ServiceUser,
                reason: RejectReason::ServiceUser(ServiceUserReason::NoReason),
            };
            assert_eq!(serialize_reject_pdu(&pdu)[7], expected, "result {:?} should encode as {:#04x}", result, expected);
        }
    }

    #[test]
    fn test_serialize_reject_pdu_encodes_all_reasons_correctly() {
        let cases: &[(RejectSource, RejectReason, u8)] = &[
            (RejectSource::ServiceUser, RejectReason::ServiceUser(ServiceUserReason::NoReason), 1),
            (RejectSource::ServiceUser, RejectReason::ServiceUser(ServiceUserReason::ApplicationContextNameNotSupported), 2),
            (RejectSource::ServiceUser, RejectReason::ServiceUser(ServiceUserReason::CallingAeNotRecognized), 3),
            (RejectSource::ServiceUser, RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized), 7),
            (RejectSource::Acse, RejectReason::Acse(AcseReason::NoReason), 1),
            (RejectSource::Acse, RejectReason::Acse(AcseReason::ProtocolNotSupported), 2),
            (RejectSource::Presentation, RejectReason::Presentation(PresentationReason::TemporaryCongestion), 1),
            (RejectSource::Presentation, RejectReason::Presentation(PresentationReason::LocalLimitExceeded), 2),
        ];
        for &(source, reason, expected) in cases {
            let pdu = AssociateRjPdu {
                result: AssociationResult::RejectedPermanent,
                source,
                reason,
            };
            assert_eq!(serialize_reject_pdu(&pdu)[9], expected, "reason {:?} should encode as {:#04x}", reason, expected);
        }
    }

    #[test]
    fn test_serialize_reject_pdu_roundtrip() {
        let cases: &[(AssociationResult, RejectSource, RejectReason)] = &[
            (AssociationResult::RejectedPermanent, RejectSource::ServiceUser, RejectReason::ServiceUser(ServiceUserReason::NoReason)),
            (AssociationResult::RejectedTransient, RejectSource::Acse, RejectReason::Acse(AcseReason::ProtocolNotSupported)),
            (AssociationResult::RejectedPermanent, RejectSource::Presentation, RejectReason::Presentation(PresentationReason::LocalLimitExceeded)),
        ];
        for &(result, source, reason) in cases {
            let pdu = AssociateRjPdu { result, source, reason };
            let bytes = serialize_reject_pdu(&pdu);
            let mut reader = Cursor::new(bytes);
            let recovered = deserialize_reject_pdu(&mut reader).expect("roundtrip deserialization must succeed");
            assert_eq!(recovered.result, result);
            assert_eq!(recovered.source, source);
            assert_eq!(recovered.reason, reason);
        }
    }
}
