use std::fmt;
use std::io::{BufRead, BufReader, Read};
use std::string::String;

use crate::Result;
use crate::associate::presentation_context::{SyntaxItemBuilder, PresentationContextResult, serialize_presentation_context_item, deserialize_presentation_context_item};
use crate::associate::{ITEM_LENGTH_LENGTH, next_byte_item_type, AssociationItemType, MaximumLength, UserInformation, presentation_context::{PresentationContextItem, PresentationContextItemBuilder}};
use crate::pdu::{PDU_LENGTH_LENGTH, PDU_TYPE_LENGTH, PduType, read_padding, vec8_add_padding};

/// Length of the Protocol Version field in a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU
const PROTOCOL_VERSION_LENGTH: usize = 2;

/// Length of the Called-AE and Calling-AE fields in a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU
const AE_LENGTH: usize = 16;

/// Length of sub-items without the variable field.
const SUB_ITEM_NO_VARIABLE_FIELDS_LENGTH: u16 = 4;

/// Events related to A-ASSOCIATE. Events lead to actions defined by the DICOM standard.
///
/// ISO/TR 2382:2015 defines primitives. Primitives are abstract interactions between a service user and a service provider.
/// In DICOM, primitives are interactions between the DICOM server (service provider) and the client (service user).
///
/// See [DICOM standard part 8 subsection 9](https://dicom.nema.org/medical/dicom/current/output/html/part08.html#sect_9).
enum AssociationEvent {
    PrimitiveRequestAssociation,
    PrimitiveResponseAccept,
    PrimitiveResponseReject,
    PrimitiveConfirmTransport,
    PrimitiveIndicationTransport,
    AssociationRequest,
    AssociationAccept,
    AssociationReject,
}

#[derive(Debug)]
pub enum Error {
    InvalidValue,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidValue => write!(f, "Invalid AssociationItemType value"),
        }
    }
}

impl std::error::Error for Error {}

pub struct AssociateRqAcPdu {
    pub pdu_type: PduType,
    pub length: u32,
    pub protocol_version: u16,
    called_ae: String,
    calling_ae: String,
    pub application_context_item: ApplicationContextItem,
    pub presentation_context_items: Vec<PresentationContextItem>,
    user_info_item: UserInfoItem,
}

// TODO: Add builder; several presentation context items; new_rq free function
// TODO: move new_rq to free function, add new
impl AssociateRqAcPdu {
    pub fn new_rq(called_ae: &str, calling_ae: &str) -> Result<Self> {
        // 68 is the length of the A-ASSOCIATE-RQ/AC PDU minus the variable fields
        const NO_VARIABLE_FIELDS_LENGTH: u32 = 68;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let application_context_item = ApplicationContextItem::new();
        length += application_context_item.item_length();

        let mut presentation_context_items: Vec<PresentationContextItem> = Vec::new();

        presentation_context_items.push(
            PresentationContextItemBuilder::new()
                .item_type(AssociationItemType::PresentationContextRq)
                .context_id(1)
                .abstract_syntax_item(SyntaxItemBuilder::new()
                    .item_type(AssociationItemType::AbstractSyntax)
                    .syntax("1.2.840.10008.1.1")
                    .build()?
                )
                .add_transfer_syntax(SyntaxItemBuilder::new()
                    .item_type(AssociationItemType::TransferSyntax)
                    .syntax("1.2.840.10008.1.2")
                    .build()?
                )
                .build()?
        );

        length += presentation_context_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        let user_info_item = UserInfoItem::new();

        length += user_info_item.item_length();

        Ok(Self {
            pdu_type: PduType::AssociateRequest,
            length,
            protocol_version: 1,
            called_ae: called_ae.into(),
            calling_ae: calling_ae.into(),
            application_context_item,
            presentation_context_items,
            user_info_item,
        })
    }

    pub fn new_ac(called_ae: &str, calling_ae: &str) -> Result<Self> {
        // 68 is the length of the A-ASSOCIATE-RQ/AC PDU minus the variable fields
        const NO_VARIABLE_FIELDS_LENGTH: u32 = 68;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let application_context_item = ApplicationContextItem::new();
        length += application_context_item.item_length();

        let mut presentation_context_items: Vec<PresentationContextItem> = Vec::new();

        presentation_context_items.push(
            PresentationContextItemBuilder::new()
                .item_type(AssociationItemType::PresentationContextAc)
                .context_id(1)
                .result(PresentationContextResult::Acceptance)
                .abstract_syntax_item(SyntaxItemBuilder::new()
                    .item_type(AssociationItemType::AbstractSyntax)
                    .syntax("1.2.840.10008.1.1")
                    .build()?
                )
                .add_transfer_syntax(SyntaxItemBuilder::new()
                    .item_type(AssociationItemType::TransferSyntax)
                    .syntax("1.2.840.10008.1.2")
                    .build()?
                )
                .build()?
        );

        length += presentation_context_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        let user_info_item = UserInfoItem::new();

        length += user_info_item.item_length();

        Ok(Self {
            pdu_type: PduType::AssociateAccept,
            length,
            protocol_version: 1,
            called_ae: called_ae.into(),
            calling_ae: calling_ae.into(),
            application_context_item,
            presentation_context_items,
            user_info_item,
        })
    }

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
        &self.user_info_item.sub_items()
    }
}

/// TODO: Look up byteorder and use writer
pub fn serialize_association_pdu(request: &AssociateRqAcPdu) -> Result<Vec<u8>> {
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
    pdu.extend(std::iter::repeat(0x20).take(16 - len));



    // Calling application entity
    let ae = request.calling_ae().as_bytes();
    let len = ae.len().min(16);

    pdu.extend_from_slice(&ae[..len]);
    pdu.extend(std::iter::repeat(0x20).take(16 - len));


    vec8_add_padding(&mut pdu, 32);

    pdu.extend(serialize_application_context_item(
        &request.application_context_item,
    )?);

    for item in request.presentation_context_items.iter() {
        pdu.extend(serialize_presentation_context_item(item)?);
    }

    pdu.extend(serialize_user_info_item(&request.user_info_item)?);

    Ok(pdu)
}

/// Deserializes a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU. Takes a reader of u8
pub fn deserialize_association_pdu<T: Read>(reader: &mut T) -> Result<AssociateRqAcPdu> {
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
                .ok_or_else(|| "Invalid item type".to_string())?,
        )?;

        match next_type {
            AssociationItemType::ApplicationContext => {
                application_context_item = Some(deserialize_application_context_item(&mut reader)?);
            }

            AssociationItemType::PresentationContextAc
            | AssociationItemType::PresentationContextRq => {
                presentation_context_items
                    .push(deserialize_presentation_context_item(&mut reader)?);
            }

            AssociationItemType::UserInformation => {
                user_info_item = Some(deserialize_user_info_item(&mut reader)?);
            }

            _ => {
                return Err("Invalid item type".into());
            }
        }
    }

    // TODO: Handle Some for called_ae and calling_ae
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
pub struct ApplicationContextItem {
    pub item_type: u8,
    pub length: u16,
    context_name: String,
}

impl ApplicationContextItem {
    pub fn new() -> Self {
        const APPLICATION_CONTEXT_NAME: &'static str = "1.2.840.10008.3.1.1.1";

        Self {
            item_type: 0x10,
            length: APPLICATION_CONTEXT_NAME.len() as u16,
            context_name: APPLICATION_CONTEXT_NAME.into(),
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

fn serialize_application_context_item(item: &ApplicationContextItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(&item.context_name.as_bytes());

    Ok(pdu)
}

fn deserialize_application_context_item<T: Read>(reader: &mut T) -> Result<ApplicationContextItem> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    let mut context_name = vec![0u8; length as usize];
    reader.read_exact(&mut context_name)?;

    Ok(ApplicationContextItem {
        item_type: pdu_type[0].into(),
        length,
        context_name: String::from_utf8(context_name)?,
    })
}

pub struct UserInfoItem {
    pub item_type: AssociationItemType,
    pub length: u16,
    pub sub_items: Vec<UserInformationSubItem>,
}

impl UserInfoItem {
    pub fn new() -> Self {
        let mut length = 0;

        let mut sub_items: Vec<UserInformationSubItem> = Vec::new();

        // Mandatory Maximum Length Sub-Item
        sub_items.push(UserInformationSubItem::new(
            0x51,
            UserInformation::MaximumLength(MaximumLength {
                maximum_length: 300,
            }),
        ));

        length += sub_items.iter().map(|item| item.item_length()).sum::<u32>();

        Self {
            item_type: AssociationItemType::UserInformation,
            length: length as u16,
            sub_items,
        }
    }

    pub fn item_length(&self) -> u32 {
        const USER_ITEM_DEFAULT_LENGTH: u32 = 4;
        USER_ITEM_DEFAULT_LENGTH + self.length as u32
    }

    pub fn sub_items(&self) -> &Vec<UserInformationSubItem> {
        &self.sub_items
    }
}

fn serialize_user_info_item(item: &UserInfoItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&item.length.to_be_bytes());

    for item in item.sub_items.iter() {
        pdu.extend(serialize_sub_item(&item)?);
    }

    Ok(pdu)
}

fn deserialize_user_info_item<T: Read>(reader: &mut T) -> Result<UserInfoItem> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    // Split reader into subreader which is expected to contain the rest of the contents presentation context item contents.
    let mut sub_item_reader = BufReader::new(reader.take(length as u64));

    let mut sub_items: Vec<UserInformationSubItem> = Vec::new();

    while !sub_item_reader.fill_buf()?.is_empty() {
        sub_items.push(deserialize_sub_item(&mut sub_item_reader)?);
    }

    Ok(UserInfoItem {
        item_type: pdu_type[0].try_into()?,
        length,
        sub_items,
    })
}

pub struct UserInformationSubItem {
    pub item_type: u8,
    pub length: u16,
    pub inner: UserInformation,
}

impl UserInformationSubItem {
    pub fn new(item_type: u8, inner: UserInformation) -> Self {
        Self {
            item_type,
            length: match inner {
                UserInformation::MaximumLength(_) => 4,
                _ => 2,
            },
            inner,
        }
    }

    pub fn item_length(&self) -> u32 {
        const USER_ITEM_DEFAULT_LENGTH: u32 = 4;
        USER_ITEM_DEFAULT_LENGTH + self.length as u32
    }

    pub fn item_type(&self) -> u8 {
        self.item_type
    }

    pub fn inner(&self) -> &UserInformation {
        &self.inner
    }
}

fn serialize_sub_item(item: &UserInformationSubItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();
    pdu.push(item.item_type);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());

    match &item.inner {
        UserInformation::MaximumLength(user_item) => {
            pdu.extend_from_slice(&user_item.maximum_length.to_be_bytes());
        }
    }

    Ok(pdu)
}

fn deserialize_sub_item<T: Read>(reader: &mut T) -> Result<UserInformationSubItem> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;
    let item_type = pdu_type[0];

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    let mut value = vec![0u8; length as usize];
    reader.read_exact(&mut value)?;

    Ok(UserInformationSubItem {
        item_type,
        length,
        inner: match item_type {
            0x51 => {
                // TODO: Figure out how to make expect look better
                let arr: [u8; 4] = value[..4]
                    .try_into()
                    .expect("slice must be exactly 4 bytes");
                let maximum_length = u32::from_be_bytes(arr);
                UserInformation::MaximumLength(MaximumLength { maximum_length })
            }
            _ => {
                todo!();
            }
        },
    })
}
