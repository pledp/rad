mod abort;
pub mod presentation_context;
pub mod rj;
mod rq_ac;
mod user_information;

pub use abort::*;
pub use rq_ac::*;
pub use user_information::*;

use crate::Result;
use rj::{AcseReason, RejectReason, RejectSource};

/// Length of the Item length field.
pub(self) const ITEM_LENGTH_LENGTH: usize = 2;

/// Length of the Presentation Context ID field of the Presentation Context Item.
pub(self) const CONTEXT_ID_LENGTH: usize = 1;

/// Length of the Result/Reason field.
pub(self) const RESULT_LENGTH: usize = 1;

fn service_provider_rq_pdu_validation(
    pdu: &AssociateRqAcPdu,
) -> Option<(RejectedAssociationResult, RejectReason)> {
    let source = RejectSource::Acse;

    if pdu.protocol_version != 1 {
        return Some((
            RejectedAssociationResult::RejectedPermanent,
            RejectReason::Acse(AcseReason::ProtocolNotSupported),
        ));
    }

    todo!();
}

#[derive(Debug)]
pub enum RejectedAssociationResult {
    RejectedPermanent,
    RejectedTransient,
}

/// Peek into the next byte and output item type.
fn next_byte_item_type<T>(item_type: T) -> Result<AssociationItemType>
where
    T: TryInto<AssociationItemType>,
    <T as TryInto<AssociationItemType>>::Error: std::error::Error + Send + Sync + 'static,
{
    item_type
        .try_into()
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AssociationItemType {
    ApplicationContext,
    PresentationContextRq,
    PresentationContextAc,
    UserInformation,
    AbstractSyntax,
    TransferSyntax,
}

impl TryFrom<u8> for AssociationItemType {
    type Error = crate::associate::Error;

    fn try_from(value: u8) -> std::result::Result<Self, Error> {
        match value {
            0x10 => Ok(AssociationItemType::ApplicationContext),
            0x20 => Ok(AssociationItemType::PresentationContextRq),
            0x21 => Ok(AssociationItemType::PresentationContextAc),
            0x30 => Ok(AssociationItemType::AbstractSyntax),
            0x40 => Ok(AssociationItemType::TransferSyntax),
            0x50 => Ok(AssociationItemType::UserInformation),
            _ => Err(Error::InvalidValue),
        }
    }
}

impl From<AssociationItemType> for u8 {
    fn from(value: AssociationItemType) -> Self {
        match value {
            AssociationItemType::ApplicationContext => 0x10,
            AssociationItemType::PresentationContextRq => 0x20,
            AssociationItemType::PresentationContextAc => 0x21,
            AssociationItemType::AbstractSyntax => 0x30,
            AssociationItemType::TransferSyntax => 0x40,
            AssociationItemType::UserInformation => 0x50,
        }
    }
}
