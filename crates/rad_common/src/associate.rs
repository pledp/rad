use std::convert::TryFrom;
use std::io::Read;
use std::string::String;

use crate::pdu::{ PduType, vec8_add_padding, SerializableItem };
use crate::Result;

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

pub struct AssociateRequestAcceptPdu {
    pub pdu_type: PduType,
    pub length: u32,
    pub protocol_version: u16,
    pub called_ae: String,
    pub calling_ae: String,
    pub application_context_item: ApplicationContextItem,
    pub presentation_context_items: Vec<PresentationContextItem>,
    pub user_info_item: UserInfoItem,
}

// TODO: Add builder; several presentation context items
impl AssociateRequestAcceptPdu {
    pub fn new_rq(called_ae: &str, calling_ae: &str) -> Self {
        // 68 is the length of the A-ASSOCIATE-RQ/AC PDU minus the variable fields
        const NO_VARIABLE_FIELDS_LENGTH: u32 = 68;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let application_context_item = ApplicationContextItem::new();
        length += application_context_item.item_length();

        let mut presentation_context_items: Vec<PresentationContextItem> = Vec::new();

        let uids: Vec<&str> = vec!["1.2.840.10008.1.2"];

        presentation_context_items.push(PresentationContextItem::new_rq(
            1,
            "1.2.840.10008.1.1",
            uids,
        ));

        length += presentation_context_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        let user_info_item = UserInfoItem::new();

        length += user_info_item.item_length();

        Self {
            pdu_type: PduType::AssociateRequest,
            length,
            protocol_version: 1,
            called_ae: called_ae.into(),
            calling_ae: calling_ae.into(),
            application_context_item,
            presentation_context_items,
            user_info_item,
        }
    }
}

// TODO: item_type struct
struct ApplicationContextItem {
    pub item_type: u8,
    pub length: u16,
    pub context_name: String,
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
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum PresentationContextItemType {
    Rq = 0x20,
    Ac = 0x30,
}

impl TryFrom<u8> for PresentationContextItemType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x20 => Ok(PresentationContextItemType::Rq),
            0x21 => Ok(PresentationContextItemType::Ac),
            _ => Err("Invalid valie".into()),
        }
    }
}

impl From<PresentationContextItemType> for u8 {
    fn from(value: PresentationContextItemType) -> Self {
        match value {
            PresentationContextItemType::Rq => 0x20,
            PresentationContextItemType::Ac => 0x21,
        }
    }
}

struct PresentationContextItem {
    pub item_type: PresentationContextItemType,
    pub length: u16,
    pub context_id: u8,
    pub result: Option<AssociateResult>,
    pub abstract_syntax_item: Option<SyntaxItem>,
    pub transfer_syntax_items: Vec<SyntaxItem>,
}

impl PresentationContextItem {
    fn new_rq(context_id: u8, abstract_syntax: &str, transfer_syntax: Vec<&str>) -> Self {
        // Presentation context length without variable fields is 4
        const NO_VARIABLE_FIELDS_LENGTH: u16 = 4;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let abstract_syntax_item = SyntaxItem::new(
            SyntaxItemType::AbstractSyntax,
            abstract_syntax
        );

        length += abstract_syntax_item.item_length() as u16;

        let mut transfer_syntax_items: Vec<SyntaxItem> = Vec::new();
        for item in transfer_syntax {
            let syntax_item = SyntaxItem::new(
                SyntaxItemType::TransferSyntax,
                item,
            );

            length += syntax_item.item_length() as u16;

            transfer_syntax_items.push(syntax_item);
        }

        Self {
            item_type: PresentationContextItemType::Rq,
            length,
            context_id,
            result: None,
            abstract_syntax_item: Some(abstract_syntax_item),
            transfer_syntax_items,
        }

    }

    fn new_ac(result: AssociateResult) -> Self {
        todo!();
    }

    pub fn item_length(&self ) -> u32 {
        // Length field does not include first 4 bytes of item
        const PRESENTATION_ITEM_INCLUSIVE_LENGTH: u16 = 4;
        (PRESENTATION_ITEM_INCLUSIVE_LENGTH + self.length) as u32
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum AssociateResult {
    Acceptance,
    UserRejection,
    NoReason,
    AbstractSyntaxNotSupported,
    TransferSyntaxesNotSupported,
}

impl TryFrom<u8> for AssociateResult {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x00 => Ok(AssociateResult::Acceptance),
            0x01 => Ok(AssociateResult::UserRejection),
            0x02 => Ok(AssociateResult::NoReason),
            0x03 => Ok(Self::AbstractSyntaxNotSupported),
            0x04 => Ok(Self::TransferSyntaxesNotSupported),
            _ => Err("Invalid valie".into()),
        }
    }
}

impl From<AssociateResult> for u8 {
    fn from(value: AssociateResult) -> Self {
        match value {
            AssociateResult::Acceptance => 0x00,
            AssociateResult::UserRejection => 0x01,
            AssociateResult::NoReason => 0x02,
            AssociateResult::AbstractSyntaxNotSupported => 0x03,
            AssociateResult::TransferSyntaxesNotSupported => 0x04,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(u8)]
enum SyntaxItemType {
    AbstractSyntax,
    TransferSyntax
}

impl TryFrom<u8> for SyntaxItemType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x30 => Ok(SyntaxItemType::AbstractSyntax),
            0x40 => Ok(SyntaxItemType::TransferSyntax),
            _ => Err("Invalid valie".into()),
        }
    }
}

impl From<SyntaxItemType> for u8 {
    fn from(value: SyntaxItemType) -> Self {
        match value {
            SyntaxItemType::AbstractSyntax => 0x30,
            SyntaxItemType::TransferSyntax => 0x40,
        }
    }
}

struct SyntaxItem {
    pub item_type: SyntaxItemType,
    pub length: u16,
    pub syntax: String,
}

impl SyntaxItem {
    fn new(item_type: SyntaxItemType, syntax: &str) -> Self {
        Self {
            item_type,
            length: syntax.len() as u16,
            syntax: syntax.into(),
        }
    }

    pub fn item_length(&self) -> u32 {
        const SYNTAX_ITEM_DEFAULT_LENGTH: u32 = 4;
        println!("LENGTH: {}", SYNTAX_ITEM_DEFAULT_LENGTH + self.length as u32);

        SYNTAX_ITEM_DEFAULT_LENGTH + self.length as u32
    }
}

struct UserInfoItem {
    pub item_type: u8,
    pub length: u16,
    pub sub_items: Vec<SubItem>,
}

impl UserInfoItem {
    pub fn new() -> Self {
        let mut length = 0;

        let mut sub_items: Vec<SubItem> = Vec::new();

        // Mandatory Maximum Length Sub-Item
        sub_items.push(
            SubItem::new(
                0x51,
                SubItemType::U32(16384)
            )
        );

        length += sub_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        Self {
            item_type: 0x50,
            length: length as u16,
            sub_items,
        }
    }

    pub fn item_length(&self) -> u32 {
        const USER_ITEM_DEFAULT_LENGTH: u32 = 4;
        USER_ITEM_DEFAULT_LENGTH + self.length as u32
    }
}

impl SerializableItem for UserInfoItem {
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut pdu: Vec<u8> = Vec::new();

        pdu.push(self.item_type);
        vec8_add_padding(&mut pdu, 1);

        pdu.extend_from_slice(&self.length.to_be_bytes());

        for item in self.sub_items.iter() {
            pdu.extend(item.serialize()?);
        }

        Ok(pdu)
    }
}

enum SubItemType {
    U32(u32),
    String(String),
}

struct SubItem {
    pub item_type: u8,
    pub length: u16,
    pub data: SubItemType,
}

impl SubItem {
    pub fn new(item_type: u8, data: SubItemType) -> Self {
        Self {
            item_type,
            length: match &data {
                SubItemType::String(text) => text.len() as u16,
                SubItemType::U32(_) => 4,
            },
            data
        }
    }

    pub fn item_length(&self) -> u32 {
        const USER_ITEM_DEFAULT_LENGTH: u32 = 4;
        USER_ITEM_DEFAULT_LENGTH + self.length as u32
    }
}

impl SerializableItem for SubItem {
    fn serialize(&self) -> Result<Vec<u8>> {
        let mut pdu: Vec<u8> = Vec::new();
        pdu.push(self.item_type);
        vec8_add_padding(&mut pdu, 1);
        pdu.extend_from_slice(&self.length.to_be_bytes());

        match &self.data {
            SubItemType::String(text) => pdu.extend_from_slice(text.as_bytes()),
            SubItemType::U32(value) => pdu.extend_from_slice(&value.to_be_bytes()),
        }

        Ok(pdu)
    }
}

pub fn read_associate_rq_ac<R: Read>(mut reader: R) -> Result<AssociateRequestAcceptPdu> {
    todo!();
    /*
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf);

    let pdu_type: PduType = type_buf[0].try_into()?;

    Ok(AAssociateRq {
        pdu_type: PduType::AssociateRq,
        length: 2,
        called_ae: "test".into(),
        calling_ae: "test2".into(),
    })
    */
}

pub fn serialize_association_rq_ac(request: &AssociateRequestAcceptPdu) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(0x01);

    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&request.length.to_be_bytes());
    pdu.extend_from_slice(&request.protocol_version.to_be_bytes());

    vec8_add_padding(&mut pdu, 2);

    // Called application entity, add 0x20 as padding
    let ae = request.called_ae.as_bytes();
    let len = ae.len().min(16);

    pdu.extend_from_slice(&ae[..len]);
    pdu.extend(std::iter::repeat(0x20).take(16 - len));

    // Calling application entity
    let ae = request.calling_ae.as_bytes();
    let len = ae.len().min(16);

    pdu.extend_from_slice(&ae[..len]);
    pdu.extend(std::iter::repeat(0x20).take(16 - len));

    vec8_add_padding(&mut pdu, 32);

    pdu.extend(
        serialize_application_context_item(&request.application_context_item)?
    );

    for item in request.presentation_context_items.iter() {
        pdu.extend(serialize_presentation_context_item(item)?);
    }

    pdu.extend(request.user_info_item.serialize()?);

    Ok(pdu)
}

fn serialize_application_context_item(item: &ApplicationContextItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(0x10);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(&item.context_name.as_bytes());

    Ok(pdu)
}

fn serialize_presentation_context_item(item: &PresentationContextItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.push(item.context_id);

    vec8_add_padding(&mut pdu, 1);

    // Add result if it exists
    if let Some(result) = item.result.clone() {
        pdu.push(result.into());
    }
    else {
        pdu.push(0xff);
    }

    vec8_add_padding(&mut pdu, 1);

    if let Some(item) = &item.abstract_syntax_item {
        pdu.extend(serialize_syntax_item(&item)?);
    }

    for item in item.transfer_syntax_items.iter() {
        pdu.extend(serialize_syntax_item(&item)?);
    }

    Ok(pdu)
}

fn serialize_syntax_item(item: &SyntaxItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(&item.syntax.as_bytes());

    Ok(pdu)
}

/// Deserializes a A-ASSOCIATE-RQ or A-ASSOCIATE-AC PDU. Takes a reader of u8
pub fn deserialize_association_pdu<T: Read>(reader: &mut T) -> Result<AssociateRequestAcceptPdu> {
    todo!()
}
