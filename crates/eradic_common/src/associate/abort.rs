use std::io::Read;

use thiserror::Error;

use crate::{associate::PduDeserializationError, pdu::{PDU_HEADER_LENGTH, PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding, vec8_add_padding}};

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
    fn new(
        pdu_type: PduType,
        source: AbortSource,
        reason: AbortReason,
    ) -> Self {
        Self {
            pdu_type,
            length: 4,
            source,
            reason
        }
    }
}

pub(crate) fn serialize_abort_pdu(item: &AssociateAbortPdu) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.pdu_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());

    vec8_add_padding(&mut pdu, 2);
    pdu.push(item.source.into());
    pdu.push(item.reason.into());

    pdu
}

pub(crate) fn deserialize_abort_pdu<T: Read>(
    reader: &mut T,
) -> Result<AssociateAbortPdu, PduDeserializationError> {
    const SOURCE_LENGTH: usize = 1;
    const REASON_LENGTH: usize = 1;

    let mut item_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut item_type)?;

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
        item_type[0]
            .try_into()
            .map_err(|_| PduDeserializationError::InvalidItemType)?,
        source[0].try_into()?,
        reason[0].try_into()?
    ))
}
