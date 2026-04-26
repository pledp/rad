pub mod abort;
pub mod presentation_context;
pub mod rj;
mod rq_ac;
mod user_information;

use std::io::Read;

use thiserror::Error;

pub use rq_ac::*;
pub use user_information::*;

use crate::{DeserializedPdu, Result, associate::abort::deserialize_abort_pdu, event::Event, pdu::PduType};

/// Length of the Item length field.
const ITEM_LENGTH_LENGTH: usize = 2;

/// Length of the Presentation Context ID field of the Presentation Context Item.
const CONTEXT_ID_LENGTH: usize = 1;

/// Length of the Result/Reason field.
const RESULT_LENGTH: usize = 1;

#[derive(Debug, PartialEq)]
pub enum RejectedAssociateResult {
    RejectedPermanent,
    RejectedTransient,
}

/// Peek into the next byte and output item type.
fn next_byte_item_type<T>(item_type: T) -> Result<AssociateItemType>
where
    T: TryInto<AssociateItemType>,
    <T as TryInto<AssociateItemType>>::Error: std::error::Error + Send + Sync + 'static,
{
    item_type
        .try_into()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AssociateItemType {
    ApplicationContext,
    PresentationContextRq,
    PresentationContextAc,
    UserInformation,
    AbstractSyntax,
    TransferSyntax,
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
            _ => Err(PduDeserializationError::InvalidItemType),
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
        }
    }
}

#[derive(Debug, Error)]
pub enum PduDeserializationError {
    #[error("Item type does not exist")]
    InvalidItemType,
    #[error(transparent)]
    InvalidSyntaxItem(#[from] presentation_context::SyntaxItemError),
    #[error(transparent)]
    InvalidLength(#[from] std::io::Error),
    #[error(transparent)]
    InvalidEncoding(#[from] std::string::FromUtf8Error),
    #[error(transparent)]
    InvalidAbortParam(#[from] abort::AbortParseError)
}

pub fn deserialized_pdu_from_reader<R>(reader: &mut R, pdu_type: PduType) -> Result<DeserializedPdu>
where
    R: Read
{
    Ok(match pdu_type {
        PduType::AssociateRequest => {
            DeserializedPdu::AssociateRequest(
                deserialize_Associate_pdu(reader)?
            )
        },
        PduType::Abort => {
            DeserializedPdu::Abort(
                deserialize_abort_pdu(reader)?
            )
        }
        _ => todo!(),
    })
}

pub fn event_from_deserialized_pdu(pdu: DeserializedPdu) -> Event {
    match pdu {
        DeserializedPdu::AssociateRequest(inner) => Event::AssociateRequestPdu(inner),
        DeserializedPdu::Abort(inner) => Event::AssociateAbortPdu(inner),
        _ => todo!()
    }
}
