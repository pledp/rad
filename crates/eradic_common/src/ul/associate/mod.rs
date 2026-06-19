pub mod abort;
pub mod presentation_context;
pub mod rj;
pub mod rq_ac;
pub mod syntax;
pub mod user_information;

use std::io::Read;

use strum_macros::Display;
use thiserror::Error;

use crate::{
    DeserializedPdu,
    pdu::PduType,
    ul::associate::{
        abort::deserialize_abort_pdu, presentation_context::PresentationContextError, rj::deserialize_reject_pdu, rq_ac::deserialize_associate_pdu, syntax::SyntaxItemError, user_information::UserInfoItemError
    },
};

/// Length of the Item length field.
const ITEM_LENGTH_LENGTH: usize = 2;

/// Length of the Presentation Context ID field of the Presentation Context Item.
const CONTEXT_ID_LENGTH: usize = 1;

/// Length of the Result/Reason field.
const RESULT_LENGTH: usize = 1;

#[derive(Debug, Error)]
pub enum AssociationResultError {
    #[error("invalid association result value: {0}")]
    InvalidValue(u8),
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AssociationResult {
    Accepted,
    RejectedPermanent,
    RejectedTransient,
}

impl TryFrom<u8> for AssociationResult {
    type Error = AssociationResultError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(AssociationResult::Accepted),
            0x01 => Ok(AssociationResult::RejectedPermanent),
            0x02 => Ok(AssociationResult::RejectedTransient),
            v => Err(AssociationResultError::InvalidValue(v)),
        }
    }
}

impl From<AssociationResult> for u8 {
    fn from(value: AssociationResult) -> Self {
        match value {
            AssociationResult::Accepted => 0x00,
            AssociationResult::RejectedPermanent => 0x01,
            AssociationResult::RejectedTransient => 0x02,
        }
    }
}

/// Peek into the next byte and output item type.
fn next_byte_item_type<T>(item_type: T) -> Result<AssociateItemType, PduDeserializationError>
where
    T: TryInto<AssociateItemType, Error = PduDeserializationError>,
{
    item_type.try_into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[repr(u8)]
pub enum AssociateItemType {
    ApplicationContext,
    PresentationContextRq,
    PresentationContextAc,
    UserInformation,
    AbstractSyntax,
    TransferSyntax,
    MaximumLength,
    ImplementationClassUid,
}

impl TryFrom<u8> for AssociateItemType {
    type Error = PduDeserializationError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x10 => Ok(AssociateItemType::ApplicationContext),
            0x20 => Ok(AssociateItemType::PresentationContextRq),
            0x21 => Ok(AssociateItemType::PresentationContextAc),
            0x30 => Ok(AssociateItemType::AbstractSyntax),
            0x40 => Ok(AssociateItemType::TransferSyntax),
            0x50 => Ok(AssociateItemType::UserInformation),
            0x51 => Ok(AssociateItemType::MaximumLength),
            0x52 => Ok(AssociateItemType::ImplementationClassUid),
            _ => Err(PduDeserializationError::UnrecognizedItemType(value)),
        }
    }
}

impl From<AssociateItemType> for u8 {
    fn from(value: AssociateItemType) -> Self {
        match value {
            AssociateItemType::ApplicationContext => 0x10,
            AssociateItemType::PresentationContextRq => 0x20,
            AssociateItemType::PresentationContextAc => 0x21,
            AssociateItemType::AbstractSyntax => 0x30,
            AssociateItemType::TransferSyntax => 0x40,
            AssociateItemType::UserInformation => 0x50,
            AssociateItemType::MaximumLength => 0x51,
            AssociateItemType::ImplementationClassUid => 0x52,
        }
    }
}

#[derive(Debug, Error)]
pub enum PduDeserializationError {
    // Unrecognized errors
    #[error("Item type does not exist: {0}")]
    UnrecognizedItemType(u8),
    #[error("Invalid PDU type: {0}")]
    UnrecognizedPduType(u8),

    // Unexpected errors
    #[error("Unexpected PDU type: {0:?}")]
    UnexpectedPduType(PduType),

    // Invalid errors
    #[error(transparent)]
    InvalidSyntaxItem(#[from] SyntaxItemError),
    #[error(transparent)]
    InvalidPresentationItem(#[from] PresentationContextError),
    #[error(transparent)]
    InvalidUserInfoItem(#[from] UserInfoItemError),
    #[error(transparent)]
    InvalidAbortPdu(#[from] abort::AbortParseError),
    #[error(transparent)]
    InvalidRejectPdu(rj::RejectParseError),
    #[error(transparent)]
    InvalidLength(#[from] std::io::Error),
    #[error(transparent)]
    InvalidEncoding(#[from] std::string::FromUtf8Error),
}

pub fn deserialized_pdu_from_reader<R>(
    reader: &mut R,
    pdu_type: PduType,
) -> std::result::Result<DeserializedPdu, PduDeserializationError>
where
    R: Read,
{
    Ok(match pdu_type {
        PduType::AssociateRequest => {
            DeserializedPdu::AssociateRequest(deserialize_associate_pdu(reader)?)
        }
        PduType::Abort => DeserializedPdu::Abort(deserialize_abort_pdu(reader)?),
        PduType::AssociateAccept => DeserializedPdu::AssociateAccept(deserialize_associate_pdu(reader)?),
        PduType::AssociateReject => DeserializedPdu::AssociateReject(deserialize_reject_pdu(reader)?),
        _ => todo!()
    })
}
