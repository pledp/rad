use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::string::String;

use thiserror::Error;

use log::error;

use crate::ul::associate::{PduDeserializationError, presentation_context};
use crate::ul::associate::presentation_context::{
    SyntaxItemBuilder, deserialize_presentation_context_item, serialize_presentation_context_item,
};
use crate::ul::associate::user_information::{
    UserInfoItem, UserInformationSubItem, deserialize_user_info_item, serialize_user_info_item,
};
use crate::ul::associate::{
    AssociateItemType, ITEM_LENGTH_LENGTH, next_byte_item_type,
    presentation_context::{PresentationContextItem, PresentationContextItemBuilder},
};
use crate::pdu::{PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding, vec8_add_padding};

/// Length of the Protocol Version field in a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU
const PROTOCOL_VERSION_LENGTH: usize = 2;

/// Length of the Called-AE and Calling-AE fields in a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU
const AE_LENGTH: usize = 16;

/// Length of sub-items without the variable field.
const SUB_ITEM_NO_VARIABLE_FIELDS_LENGTH: u16 = 4;

#[derive(Debug, Error)]
pub enum AssociateRqAcPduError {
    #[error("Transfer syntax result list must be ")]
    TransferSyntaxInvalidLength,
    #[error(transparent)]
    InvalidSyntaxItem(#[from] presentation_context::SyntaxItemError),
    #[error(transparent)]
    PresentationContextError(#[from] presentation_context::PresentationContextError),
}

#[derive(Debug, PartialEq)]
pub struct AssociateRqAcPdu {
    pub pdu_type: PduType,
    pub length: u32,
    pub protocol_version: u16,
    pub(crate) called_ae: String,
    pub(crate) calling_ae: String,
    pub application_context_item: ApplicationContextItem,
    pub presentation_context_items: Vec<PresentationContextItem>,
    pub(crate) user_info_item: UserInfoItem,
}

impl AssociateRqAcPdu {
    pub fn pdu_type(&self) -> PduType {
        self.pdu_type
    }

    pub fn context_name(&self) -> &str {
        self.application_context_item.context_name()
    }

    pub fn called_ae(&self) -> &str {
        &self.called_ae
    }

    pub fn calling_ae(&self) -> &str {
        &self.calling_ae
    }

    pub fn presentation_context_items(&self) -> &Vec<PresentationContextItem> {
        &self.presentation_context_items
    }

    pub fn user_information(&self) -> &Vec<UserInformationSubItem> {
        self.user_info_item.sub_items()
    }
}

/// TODO: Look up byteorder and use writer
pub fn serialize_associate_pdu(request: &AssociateRqAcPdu) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(request.pdu_type.into());

    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&request.length.to_be_bytes());
    pdu.extend_from_slice(&request.protocol_version.to_be_bytes());

    vec8_add_padding(&mut pdu, 2);

    // Called application entity, add 0x20 as padding
    let ae = request.called_ae().as_bytes();
    let len = ae.len().min(16);

    pdu.extend_from_slice(&ae[..len]);
    pdu.extend(std::iter::repeat_n(0x20, 16 - len));

    // Calling application entity
    let ae = request.calling_ae().as_bytes();
    let len = ae.len().min(16);

    pdu.extend_from_slice(&ae[..len]);
    pdu.extend(std::iter::repeat_n(0x20, 16 - len));

    vec8_add_padding(&mut pdu, 32);

    pdu.extend(serialize_application_context_item(
        &request.application_context_item,
    ));

    for item in request.presentation_context_items.iter() {
        pdu.extend(serialize_presentation_context_item(item));
    }

    pdu.extend(serialize_user_info_item(&request.user_info_item));

    pdu
}

/// Deserializes a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU. Takes a reader of u8
pub fn deserialize_associate_pdu<T: Read>(reader: &mut T) -> std::result::Result<AssociateRqAcPdu, PduDeserializationError> {
    let mut reader = BufReader::new(reader);

    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    read_padding(&mut reader, 1);

    let mut pdu_length = [0u8; PDU_LENGTH_LENGTH];
    reader.read_exact(&mut pdu_length)?;

    let mut protocol_version = [0u8; PROTOCOL_VERSION_LENGTH];
    reader.read_exact(&mut protocol_version)?;

    read_padding(&mut reader, 2);

    let mut called_ae = [0u8; AE_LENGTH];
    reader.read_exact(&mut called_ae)?;

    let mut calling_ae = [0u8; AE_LENGTH];
    reader.read_exact(&mut calling_ae)?;

    read_padding(&mut reader, 32);

    let mut application_context_item: Option<ApplicationContextItem> = None;
    let mut presentation_context_items: Vec<PresentationContextItem> = Vec::new();
    let mut user_info_item: Option<UserInfoItem> = None;

    // While reader is not empty, deserialize items.
    // Makes item ordering flexible. Standard does not define that items must be in certain order.
    while !reader.fill_buf()?.is_empty() {
        let next_type = next_byte_item_type(
            reader
                .fill_buf()?
                .first()
                .copied()
                .unwrap()
        )?;

        match next_type {
            AssociateItemType::ApplicationContext => {
                application_context_item = Some(deserialize_application_context_item(&mut reader)?);
            }

            AssociateItemType::PresentationContextAc
            | AssociateItemType::PresentationContextRq => {
                presentation_context_items
                    .push(deserialize_presentation_context_item(&mut reader)?);
            }

            AssociateItemType::UserInformation => {
                user_info_item = Some(deserialize_user_info_item(&mut reader)?);
            }
            _ => {}
        }
    }

    Ok(AssociateRqAcPdu {
        pdu_type: pdu_type[0].try_into()?,
        length: u32::from_be_bytes(pdu_length),
        protocol_version: u16::from_be_bytes(protocol_version),
        called_ae: String::from_utf8(called_ae.trim_ascii().to_vec())?,
        calling_ae: String::from_utf8(calling_ae.trim_ascii().to_vec())?,
        application_context_item: application_context_item.unwrap(),
        presentation_context_items,
        user_info_item: user_info_item.unwrap(),
    })
}

// TODO: item_type struct
#[derive(Debug, PartialEq)]
pub struct ApplicationContextItem {
    pub item_type: AssociateItemType,
    pub length: u16,
    context_name: String,
}

impl ApplicationContextItem {
    pub fn new<S: Into<String>>(context_name: S) -> Self {
        let context_name = context_name.into();

        Self {
            item_type: AssociateItemType::ApplicationContext,
            length: context_name.len() as u16,
            context_name,
        }
    }

    pub fn item_length(&self) -> u32 {
        const APPLICATION_ITEM_DEFAULT_LENGTH: u32 = 4;

        APPLICATION_ITEM_DEFAULT_LENGTH + self.length as u32
    }

    pub fn context_name(&self) -> &str {
        &self.context_name
    }
}

fn serialize_application_context_item(item: &ApplicationContextItem) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(item.context_name.as_bytes());

    pdu
}

fn deserialize_application_context_item<T: Read>(reader: &mut T) -> Result<ApplicationContextItem, PduDeserializationError> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    let mut context_name = vec![0u8; length as usize];
    reader.read_exact(&mut context_name)?;

    Ok(ApplicationContextItem {
        item_type: pdu_type[0].try_into()?,
        length,
        context_name: String::from_utf8(context_name)?,
    })
}
