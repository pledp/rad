use std::io::Read;

use thiserror::Error;

use crate::{pdu::{PDU_HEADER_LENGTH, PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding, vec8_add_padding}};
use crate::ul::associate::PduDeserializationError;

#[derive(Debug, Error)]
pub enum AbortParseError {
    #[error("invalid abort source: {0}")]
    InvalidAbortSource(u8),

    #[error("invalid abort reason: {0}")]
    InvalidAbortReason(u8),
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum AbortSource {
    ServiceUser = 0,
    Reserved = 1,
    ServiceProvider = 2,
}

impl From<AbortSource> for u8 {
    fn from(s: AbortSource) -> Self {
        s as u8
    }
}

impl TryFrom<u8> for AbortSource {
    type Error = AbortParseError;

    fn try_from(s: u8) -> Result<Self, Self::Error> {
        match s {
            x if x == AbortSource::ServiceUser as u8 => Ok(AbortSource::ServiceUser),
            x if x == AbortSource::Reserved as u8 => Ok(AbortSource::Reserved),
            x if x == AbortSource::ServiceProvider as u8 => Ok(AbortSource::ServiceProvider),
            _ => Err(AbortParseError::InvalidAbortSource(s)),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u8)]
pub enum AbortReason {
    NoReason = 0,
    UnrecognizedPdu = 1,
    UnexpectedPdu = 2,
    Reserved = 3,
    UnrecognizedPduParam = 4,
    UnexpectedPduParam = 5,
    InvalidPduParam = 6,
}

impl From<AbortReason> for u8 {
    fn from(r: AbortReason) -> u8 {
        r as u8
    }
}

impl TryFrom<u8> for AbortReason {
    type Error = AbortParseError;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == AbortReason::NoReason as u8 => Ok(AbortReason::NoReason),
            x if x == AbortReason::UnrecognizedPdu as u8 => Ok(AbortReason::UnrecognizedPdu),
            x if x == AbortReason::UnexpectedPdu as u8 => Ok(AbortReason::UnexpectedPdu),
            x if x == AbortReason::Reserved as u8 => Ok(AbortReason::Reserved),
            x if x == AbortReason::UnexpectedPduParam as u8 => Ok(AbortReason::UnrecognizedPduParam),
            x if x == AbortReason::UnexpectedPduParam as u8 => Ok(AbortReason::UnexpectedPduParam),
            x if x == AbortReason::InvalidPduParam as u8 => Ok(AbortReason::InvalidPduParam),
            _ => Err(AbortParseError::InvalidAbortReason(v)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct AssociateAbortPdu {
    pdu_type: PduType,
    length: u32,
    pub source: AbortSource,
    pub reason: AbortReason,
}

impl AssociateAbortPdu {
    pub fn new(
        source: AbortSource,
        reason: AbortReason,
    ) -> Self {
        Self {
            pdu_type: PduType::Abort,
            length: 4,
            source,
            reason
        }
    }
}

pub fn serialize_abort_pdu(item: &AssociateAbortPdu) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.pdu_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());

    vec8_add_padding(&mut pdu, 2);
    pdu.push(item.source.into());
    pdu.push(item.reason.into());

    pdu
}

/// Deserializes bytes from a [Read] into a [AssociateAbortPdu].
///
/// # Errors
/// - Returns [PduDeserializationError::InvalidLength] if the reader does not contain enough bytes (4 + Item Length).
/// - Returns [PduDeserializationError::InvalidItemType] error if item type is not [PduType::Abort].
pub fn deserialize_abort_pdu<T: Read>(
    reader: &mut T,
) -> Result<AssociateAbortPdu, PduDeserializationError> {
    const SOURCE_LENGTH: usize = 1;
    const REASON_LENGTH: usize = 1;

    let mut item_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut item_type)?;

    let pdu_type = PduType::try_from(item_type[0])?;

    match pdu_type {
        PduType::Abort => {}
        _ => return Err(PduDeserializationError::UnexpectedPduType(pdu_type))
    };

    read_padding(reader, 1);

    let mut item_length = [0u8; PDU_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u32::from_be_bytes(item_length);

    read_padding(reader, 2);

    let mut source = [0u8; SOURCE_LENGTH];
    reader.read_exact(&mut source)?;

    let mut reason = [0u8; REASON_LENGTH];
    reader.read_exact(&mut reason)?;

    Ok(AssociateAbortPdu::new(
        source[0].try_into()?,
        reason[0].try_into()?
    ))
}

mod tests {
    use std::io::Cursor;

    use crate::{ul::associate::{PduDeserializationError, abort::{AbortParseError, AbortReason, AbortSource, deserialize_abort_pdu}}, pdu::PduType};

    #[test]
    fn test_deserialize_abort_pdu_ok() {
        let mut reader = Cursor::new(vec![0x07, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x02, 0x00]);

        let result = deserialize_abort_pdu(&mut reader);

        assert!(result.is_ok());
        let pdu = result.unwrap();

        assert_eq!(pdu.pdu_type, PduType::Abort);
        assert_eq!(pdu.length, 4);
        assert_eq!(pdu.source, AbortSource::ServiceProvider);
        assert_eq!(pdu.reason, AbortReason::NoReason);
    }

    #[test]
    fn test_deserialize_abort_pdu_invalid_type() {
        let mut reader = Cursor::new(vec![0xFF, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x02, 0x00]);

        let result = deserialize_abort_pdu(&mut reader);

        assert!(matches!(result, Err(PduDeserializationError::InvalidItemType(0xFF))));
    }

    #[test]
    fn test_deserialize_invalid_length() {
        let mut reader = Cursor::new(vec![0x07, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x02]);

        let result = deserialize_abort_pdu(&mut reader);

        assert!(matches!(result, Err(PduDeserializationError::InvalidLength(_))));
    }

    #[test]
    fn test_deserialize_abort_pdu_invalid_source() {
        let mut reader = Cursor::new(vec![0x07, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0xFF, 0x00]);
        let result = deserialize_abort_pdu(&mut reader);
        assert!(matches!(result, Err(PduDeserializationError::InvalidAbortPdu(
            AbortParseError::InvalidAbortSource(_)
        ))));
    }

    #[test]
    fn test_deserialize_abort_pdu_invalid_reason() {
        let mut reader = Cursor::new(vec![0x07, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x02, 0xFF]);
        let result = deserialize_abort_pdu(&mut reader);
        assert!(matches!(result, Err(PduDeserializationError::InvalidAbortPdu(
            AbortParseError::InvalidAbortReason(_)
        ))));
    }

}
