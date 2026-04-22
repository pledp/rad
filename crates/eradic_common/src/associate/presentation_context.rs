use std::io::{BufRead, BufReader, Read};
use std::result::Result;

use thiserror::Error;

use crate::associate::PduDeserializationError;
use crate::associate::{
    AssociationItemType, CONTEXT_ID_LENGTH, ITEM_LENGTH_LENGTH, RESULT_LENGTH, next_byte_item_type,
};
use crate::pdu::{PDU_TYPE_LENGTH, read_padding, vec8_add_padding};

/// Length of the presentation context item without the variable field.
pub const PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH: u16 = 4;

#[derive(Debug)]
pub struct PresentationContextItem {
    pub item_type: AssociationItemType,
    pub length: u16,
    pub context_id: u8,
    pub result: Option<PresentationContextResult>,
    pub abstract_syntax_item: Option<SyntaxItem>,
    pub transfer_syntax_items: Vec<SyntaxItem>,
}

impl PresentationContextItem {
    pub fn new(
        item_type: AssociationItemType,
        context_id: u8,
        result: Option<PresentationContextResult>,
        abstract_syntax_item: Option<SyntaxItem>,
        transfer_syntax_items: Vec<SyntaxItem>,
    ) -> crate::Result<Self> {
        match item_type {
            AssociationItemType::PresentationContextRq => Ok(PresentationContextItem::new_rq(
                context_id,
                abstract_syntax_item.unwrap(),
                transfer_syntax_items,
            )),
            AssociationItemType::PresentationContextAc => Ok(PresentationContextItem::new_ac(
                context_id,
                result.unwrap(),
                transfer_syntax_items,
            )),
            _ => Err("Invalid type".into()),
        }
    }

    fn new_rq(
        context_id: u8,
        abstract_syntax_item: SyntaxItem,
        transfer_syntax_items: Vec<SyntaxItem>,
    ) -> Self {
        // Presentation context length without variable fields is 4
        let mut length = PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH;

        length += abstract_syntax_item.item_length() as u16;

        for item in &transfer_syntax_items {
            length += item.item_length() as u16;
        }

        Self {
            item_type: AssociationItemType::PresentationContextRq,
            length,
            context_id,
            result: None,
            abstract_syntax_item: Some(abstract_syntax_item),
            transfer_syntax_items,
        }
    }

    fn new_ac(
        context_id: u8,
        result: PresentationContextResult,
        transfer_syntax_items: Vec<SyntaxItem>,
    ) -> Self {
        // Presentation context length without variable fields is 4
        let mut length = PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH;

        length += transfer_syntax_items[0].item_length() as u16;

        Self {
            item_type: AssociationItemType::PresentationContextAc,
            length,
            context_id,
            result: Some(result),
            abstract_syntax_item: None,
            transfer_syntax_items,
        }
    }

    pub fn item_length(&self) -> u32 {
        // Length field does not include first 4 bytes of item
        const PRESENTATION_ITEM_INCLUSIVE_LENGTH: u16 = 4;
        (PRESENTATION_ITEM_INCLUSIVE_LENGTH + self.length) as u32
    }

    pub fn abstract_syntax(&self) -> Option<&str> {
        self.abstract_syntax_item.as_ref().map(|item| item.syntax())
    }

    pub fn transfer_syntax(&self) -> Vec<&str> {
        self.transfer_syntax_items
            .iter()
            .map(|item| item.syntax())
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PresentationContextResult {
    Acceptance,
    UserRejection,
    NoReason,
    AbstractSyntaxNotSupported,
    TransferSyntaxesNotSupported,
}

impl TryFrom<u8> for PresentationContextResult {
    type Error = crate::Error;

    fn try_from(value: u8) -> crate::Result<Self> {
        match value {
            0x00 => Ok(PresentationContextResult::Acceptance),
            0x01 => Ok(PresentationContextResult::UserRejection),
            0x02 => Ok(PresentationContextResult::NoReason),
            0x03 => Ok(Self::AbstractSyntaxNotSupported),
            0x04 => Ok(Self::TransferSyntaxesNotSupported),
            _ => Err("Invalid valie".into()),
        }
    }
}

impl From<PresentationContextResult> for u8 {
    fn from(value: PresentationContextResult) -> Self {
        match value {
            PresentationContextResult::Acceptance => 0x00,
            PresentationContextResult::UserRejection => 0x01,
            PresentationContextResult::NoReason => 0x02,
            PresentationContextResult::AbstractSyntaxNotSupported => 0x03,
            PresentationContextResult::TransferSyntaxesNotSupported => 0x04,
        }
    }
}

pub struct PresentationContextItemBuilder {
    item_type: Option<AssociationItemType>,
    context_id: Option<u8>,
    result: Option<PresentationContextResult>,
    abstract_syntax_item: Option<SyntaxItem>,
    transfer_syntax_items: Vec<SyntaxItem>,
}

impl PresentationContextItemBuilder {
    pub fn new() -> Self {
        Self {
            item_type: None,
            context_id: None,
            result: None,
            abstract_syntax_item: None,
            transfer_syntax_items: Vec::new(),
        }
    }

    pub fn item_type(mut self, item_type: AssociationItemType) -> Self {
        self.item_type = Some(item_type);
        self
    }

    pub fn context_id(mut self, context_id: u8) -> Self {
        self.context_id = Some(context_id);
        self
    }

    pub fn result(mut self, result: PresentationContextResult) -> Self {
        self.result = Some(result);
        self
    }

    pub fn abstract_syntax_item(mut self, item: SyntaxItem) -> Self {
        self.abstract_syntax_item = Some(item);
        self
    }

    pub fn add_transfer_syntax(mut self, item: SyntaxItem) -> Self {
        self.transfer_syntax_items.push(item);
        self
    }

    pub fn transfer_syntax_items(mut self, items: Vec<SyntaxItem>) -> Self {
        self.transfer_syntax_items = items;
        self
    }

    pub fn build(self) -> crate::Result<PresentationContextItem> {
        PresentationContextItem::new(
            self.item_type.unwrap(),
            self.context_id.unwrap(),
            self.result,
            self.abstract_syntax_item,
            self.transfer_syntax_items,
        )
    }
}

#[derive(Debug, Error)]
pub enum SyntaxItemError {
    #[error(
        "Item type must be {:?} or {:?}",
        AssociationItemType::AbstractSyntax,
        AssociationItemType::TransferSyntax
    )]
    InvalidItemType,
}

#[derive(Debug, PartialEq)]
pub struct SyntaxItem {
    pub item_type: AssociationItemType,
    pub length: u16,
    syntax: String,
}

impl SyntaxItem {
    /// Creates a SyntaxItem.
    ///
    /// [SyntaxItem] may represent an abstract syntax or a transfer syntax.
    ///
    /// # Arguments
    /// * `item_type` - Must be [AssociationItemType::AbstractSyntax] or [AssociationItemType::TransferSyntax].
    pub fn new(item_type: AssociationItemType, syntax: &str) -> Result<Self, SyntaxItemError> {
        match item_type {
            AssociationItemType::AbstractSyntax | AssociationItemType::TransferSyntax => {}
            _ => {
                return Err(SyntaxItemError::InvalidItemType);
            }
        }

        Ok(Self {
            item_type,
            length: syntax.len() as u16,
            syntax: syntax.into(),
        })
    }

    pub fn item_length(&self) -> u32 {
        const SYNTAX_ITEM_DEFAULT_LENGTH: u32 = 4;

        SYNTAX_ITEM_DEFAULT_LENGTH + self.length as u32
    }

    pub fn syntax(&self) -> &str {
        &self.syntax
    }
}

pub struct SyntaxItemBuilder {
    item_type: Option<AssociationItemType>,
    syntax: Option<String>,
}

impl SyntaxItemBuilder {
    pub fn new() -> Self {
        Self {
            item_type: None,
            syntax: None,
        }
    }

    pub fn item_type(mut self, item_type: AssociationItemType) -> Self {
        self.item_type = Some(item_type);
        self
    }

    pub fn syntax<S: Into<String>>(mut self, syntax: S) -> Self {
        self.syntax = Some(syntax.into());
        self
    }

    pub fn build(self) -> crate::Result<SyntaxItem> {
        Ok(SyntaxItem::new(
            self.item_type.unwrap(),
            &self.syntax.unwrap(),
        )?)
    }
}

pub(crate) fn serialize_presentation_context_item(
    item: &PresentationContextItem,
) -> crate::Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());

    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.push(item.context_id);

    vec8_add_padding(&mut pdu, 1);

    // Add result if it exists
    if let Some(result) = item.result {
        pdu.push(result.into());
    } else {
        pdu.push(0xff);
    }

    vec8_add_padding(&mut pdu, 1);

    if let Some(item) = &item.abstract_syntax_item {
        pdu.extend(serialize_syntax_item(item));
    }

    for item in item.transfer_syntax_items.iter() {
        pdu.extend(serialize_syntax_item(item));
    }

    Ok(pdu)
}

/// Deserialize [PresentationContextItem] from a reader.
/// DICOM standard expects the Abstract Syntax Item to be before the Transfer Syntax Item.
/// [deserialize_presentation_context_item] does not handle correct ordering.
pub(crate) fn deserialize_presentation_context_item<T: Read>(
    reader: &mut T,
) -> crate::Result<PresentationContextItem> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let item_length = u16::from_be_bytes(item_length);

    let mut context_id = [0u8; CONTEXT_ID_LENGTH];
    reader.read_exact(&mut context_id)?;

    read_padding(reader, 1);

    let mut result = [0u8; RESULT_LENGTH];
    reader.read_exact(&mut result)?;

    read_padding(reader, 1);

    let mut abstract_syntax_item: Option<SyntaxItem> = None;
    let mut transfer_syntax_items: Vec<SyntaxItem> = Vec::new();

    // Split reader into subreader which is expected to contain the rest of the contents presentation context item contents.
    let mut syntax_reader = BufReader::new(
        reader.take((item_length - PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH) as u64),
    );

    while !syntax_reader.fill_buf()?.is_empty() {
        let next_type = next_byte_item_type(
            syntax_reader
                .fill_buf()?
                .first()
                .copied()
                .ok_or_else(|| "Invalid item type".to_string())?,
        )?;

        match next_type {
            AssociationItemType::AbstractSyntax => {
                abstract_syntax_item = Some(deserialize_syntax_item(&mut syntax_reader)?);
            }
            AssociationItemType::TransferSyntax => {
                transfer_syntax_items.push(deserialize_syntax_item(&mut syntax_reader)?);
            }

            _ => {
                return Err("Invalid item type".into());
            }
        }
    }

    PresentationContextItem::new(
        pdu_type[0].try_into()?,
        context_id[0],
        result[0].try_into().ok(),
        abstract_syntax_item,
        transfer_syntax_items,
    )
}

pub(crate) fn serialize_syntax_item(item: &SyntaxItem) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(item.syntax().as_bytes());

    pdu
}

/// Deserializes bytes from a [Read] into a [SyntaxItem].
///
/// # Errors
/// - Returns an error if the reader does not contain enough bytes (4 + Item Length).
/// - Returns an error if [AssociationItemType] is invalid.
pub(crate) fn deserialize_syntax_item<T: Read>(
    reader: &mut T,
) -> Result<SyntaxItem, PduDeserializationError> {
    let mut item_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut item_type)?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    let mut syntax = vec![0u8; length as usize];
    reader.read_exact(&mut syntax)?;

    Ok(SyntaxItem::new(
        item_type[0]
            .try_into()
            .map_err(|_| PduDeserializationError::InvalidItemType)?,
        &String::from_utf8(syntax)?,
    )?)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_syntax_item_new_ok() {
        assert!(SyntaxItem::new(AssociationItemType::AbstractSyntax, "1.2.840.10008.1.1").is_ok());
        assert!(SyntaxItem::new(AssociationItemType::TransferSyntax, "1.2.840.10008.1.2").is_ok());
    }

    #[test]
    fn test_syntax_item_length() {
        let mut data = vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ];

        let item =
            SyntaxItem::new(AssociationItemType::TransferSyntax, "1.2.840.10008.1.1").unwrap();
        assert_eq!(item.item_length(), data.len() as u32);
    }

    #[test]
    fn test_syntax_item_new_err() {
        assert!(matches!(
            SyntaxItem::new(
                AssociationItemType::PresentationContextAc,
                "1.2.840.10008.1.1"
            ),
            Err(SyntaxItemError::InvalidItemType)
        ));
        assert!(matches!(
            SyntaxItem::new(AssociationItemType::UserInformation, "1.2.840.10008.1.1"),
            Err(SyntaxItemError::InvalidItemType)
        ));
    }

    #[test]
    fn test_deserialize_syntax_item_ok() {
        let mut data = Cursor::new(vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ]);

        let item =
            SyntaxItem::new(AssociationItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap();

        assert_eq!(item, deserialize_syntax_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_syntax_item_invalid_type() {
        let mut data = Cursor::new(vec![
            0x10, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ]);

        assert!(matches!(
            deserialize_syntax_item(&mut data),
            Err(PduDeserializationError::InvalidSyntaxItem(
                SyntaxItemError::InvalidItemType
            ))
        ));
    }

    #[test]
    fn test_deserialize_syntax_item_invalid_length() {
        let mut data = Cursor::new(vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e,
        ]);

        assert!(matches!(
            deserialize_syntax_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_serialize_syntax_item_ok() {
        let mut data = vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ];

        assert_eq!(
            serialize_syntax_item(
                &SyntaxItem::new(AssociationItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap()
            ),
            data
        );
    }

    #[test]
    fn test_syntax_item_deserialize_serialize_cycle() {
        let item =
            SyntaxItem::new(AssociationItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap();
        let serialized = serialize_syntax_item(&item);
        let deserialized_item = deserialize_syntax_item(&mut Cursor::new(serialized)).unwrap();

        assert_eq!(item, deserialized_item);
    }
}
