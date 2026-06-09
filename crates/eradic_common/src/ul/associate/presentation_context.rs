use std::io::{BufRead, BufReader, Read};
use std::result::Result;

use thiserror::Error;

use crate::pdu::{PDU_TYPE_LENGTH, read_padding, vec8_add_padding};
use crate::ul::associate::PduDeserializationError;
use crate::ul::associate::syntax::{SyntaxItem, deserialize_syntax_item, serialize_syntax_item};
use crate::ul::associate::{
    AssociateItemType, CONTEXT_ID_LENGTH, ITEM_LENGTH_LENGTH, RESULT_LENGTH, next_byte_item_type,
};

/// Length of the presentation context item without the variable field.
pub const PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH: u16 = 4;

#[derive(Debug, PartialEq)]
pub struct PresentationContextItem {
    pub item_type: AssociateItemType,
    pub length: u16,
    pub context_id: u8,
    pub result: Option<PresentationContextResult>,
    pub abstract_syntax_item: Option<SyntaxItem>,
    pub transfer_syntax_items: Vec<SyntaxItem>,
}

#[derive(Debug, Error)]
pub enum PresentationContextError {
    #[error(
        "Invalid item type, must be `AssociateItemType::PresentationContextRq` or `AssociateItemType::PresentationContextAc`: {0}"
    )]
    InvalidItemType(AssociateItemType),
}

impl PresentationContextItem {
    pub fn new(
        item_type: AssociateItemType,
        context_id: u8,
        result: Option<PresentationContextResult>,
        abstract_syntax_item: Option<SyntaxItem>,
        transfer_syntax_items: Vec<SyntaxItem>,
    ) -> Result<Self, PresentationContextError> {
        match item_type {
            AssociateItemType::PresentationContextRq => Ok(PresentationContextItem::new_rq(
                context_id,
                abstract_syntax_item.unwrap(),
                transfer_syntax_items,
            )),
            AssociateItemType::PresentationContextAc => Ok(PresentationContextItem::new_ac(
                context_id,
                result.unwrap(),
                transfer_syntax_items,
            )),
            _ => Err(PresentationContextError::InvalidItemType(item_type)),
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
            item_type: AssociateItemType::PresentationContextRq,
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
            item_type: AssociateItemType::PresentationContextAc,
            length,
            context_id,
            result: Some(result),
            abstract_syntax_item: None,
            transfer_syntax_items,
        }
    }

    /// The total length of the resulting item. The total length is a superset of [PresentationContextItem::length].
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
            _ => Err("Invalid value".into()),
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
    item_type: Option<AssociateItemType>,
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

    pub fn item_type(mut self, item_type: AssociateItemType) -> Self {
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

    pub fn build(self) -> Result<PresentationContextItem, PresentationContextError> {
        PresentationContextItem::new(
            self.item_type.unwrap(),
            self.context_id.unwrap(),
            self.result,
            self.abstract_syntax_item,
            self.transfer_syntax_items,
        )
    }
}

/// Serializes a [PresentationContextItem] into a [Vec<u8>].
pub(crate) fn serialize_presentation_context_item(item: &PresentationContextItem) -> Vec<u8> {
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

    pdu
}

/// Deserializes bytes from a [Read] into a [PresentationContextItem].
///
/// # Errors
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/presentation_item_deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/syntax_deserialize_errors.md"))]
pub(crate) fn deserialize_presentation_context_item<T: Read>(
    reader: &mut T,
) -> Result<PresentationContextItem, PduDeserializationError> {
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
        let next_type = next_byte_item_type(syntax_reader.fill_buf()?.first().copied().unwrap())?;

        match next_type {
            AssociateItemType::AbstractSyntax => {
                abstract_syntax_item = Some(deserialize_syntax_item(&mut syntax_reader)?);
            }
            AssociateItemType::TransferSyntax => {
                transfer_syntax_items.push(deserialize_syntax_item(&mut syntax_reader)?);
            }
            other => {
                return Err(PduDeserializationError::UnexpectedItemType(other))
            }
        }
    }

    Ok(PresentationContextItem::new(
        pdu_type[0].try_into()?,
        context_id[0],
        result[0].try_into().ok(),
        abstract_syntax_item,
        transfer_syntax_items,
    )?)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;
    use crate::ul::associate::{AssociateItemType, PduDeserializationError};
    use crate::ul::associate::syntax::SyntaxItem;

    /// Builds raw bytes for an abstract or transfer syntax sub-item.
    fn syntax_item_bytes(item_type: u8, syntax: &str) -> Vec<u8> {
        let mut bytes = vec![item_type, 0x00];
        bytes.extend_from_slice(&(syntax.len() as u16).to_be_bytes());
        bytes.extend_from_slice(syntax.as_bytes());
        bytes
    }

    /// Builds a complete PresentationContextRq byte stream.
    fn rq_bytes(context_id: u8, abstract_syntax: &str, transfer_syntaxes: &[&str]) -> Vec<u8> {
        let abs = syntax_item_bytes(0x30, abstract_syntax);
        let mut ts_all: Vec<u8> = Vec::new();
        for ts in transfer_syntaxes {
            ts_all.extend(syntax_item_bytes(0x40, ts));
        }
        let item_length = 4u16 + abs.len() as u16 + ts_all.len() as u16;

        let mut bytes = vec![0x20, 0x00]; // PresentationContextRq, padding
        bytes.extend_from_slice(&item_length.to_be_bytes());
        bytes.extend_from_slice(&[context_id, 0x00, 0xFF, 0x00]); // context_id, pad, no-result, pad
        bytes.extend(abs);
        bytes.extend(ts_all);
        bytes
    }

    /// Builds a complete PresentationContextAc byte stream.
    fn ac_bytes(context_id: u8, result: u8, transfer_syntax: &str) -> Vec<u8> {
        let ts = syntax_item_bytes(0x40, transfer_syntax);
        let item_length = 4u16 + ts.len() as u16;

        let mut bytes = vec![0x21, 0x00]; // PresentationContextAc, padding
        bytes.extend_from_slice(&item_length.to_be_bytes());
        bytes.extend_from_slice(&[context_id, 0x00, result, 0x00]); // context_id, pad, result, pad
        bytes.extend(ts);
        bytes
    }

    #[test]
    fn test_deserialize_rq_with_abstract_and_transfer_syntax() {
        let data = rq_bytes(0x01, "1.2.840.10008.1.1", &["1.2.840.10008.1.2"]);
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();

        assert_eq!(item.item_type, AssociateItemType::PresentationContextRq);
        assert_eq!(item.context_id, 0x01);
        assert_eq!(item.result, None);
        assert_eq!(item.abstract_syntax(), Some("1.2.840.10008.1.1"));
        assert_eq!(item.transfer_syntax(), vec!["1.2.840.10008.1.2"]);
    }

    #[test]
    fn test_deserialize_ac_with_acceptance_result() {
        let data = ac_bytes(0x03, 0x00, "1.2.840.10008.1.2");
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();

        assert_eq!(item.item_type, AssociateItemType::PresentationContextAc);
        assert_eq!(item.context_id, 0x03);
        assert_eq!(item.result, Some(PresentationContextResult::Acceptance));
        assert_eq!(item.abstract_syntax(), None);
        assert_eq!(item.transfer_syntax(), vec!["1.2.840.10008.1.2"]);
    }

    #[test]
    fn test_deserialize_rq_with_multiple_transfer_syntaxes() {
        let data = rq_bytes(
            0x01,
            "1.2.840.10008.1.1",
            &["1.2.840.10008.1.2", "1.2.840.10008.1.2.1"],
        );
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();

        assert_eq!(item.abstract_syntax(), Some("1.2.840.10008.1.1"));
        assert_eq!(
            item.transfer_syntax(),
            vec!["1.2.840.10008.1.2", "1.2.840.10008.1.2.1"]
        );
    }

    #[test]
    fn test_deserialize_rq_preserves_max_context_id() {
        // context_id is a u8 and must survive the round-trip at its boundary value
        let data = rq_bytes(0xFF, "1.2.840.10008.1.1", &["1.2.840.10008.1.2"]);
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(item.context_id, 0xFF);
    }

    #[test]
    fn test_deserialize_ac_user_rejection_result() {
        let data = ac_bytes(0x01, 0x01, "1.2.840.10008.1.2");
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(item.result, Some(PresentationContextResult::UserRejection));
    }

    #[test]
    fn test_deserialize_ac_no_reason_result() {
        let data = ac_bytes(0x01, 0x02, "1.2.840.10008.1.2");
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(item.result, Some(PresentationContextResult::NoReason));
    }

    #[test]
    fn test_deserialize_ac_abstract_syntax_not_supported_result() {
        let data = ac_bytes(0x01, 0x03, "1.2.840.10008.1.2");
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(
            item.result,
            Some(PresentationContextResult::AbstractSyntaxNotSupported)
        );
    }

    #[test]
    fn test_deserialize_ac_transfer_syntaxes_not_supported_result() {
        let data = ac_bytes(0x01, 0x04, "1.2.840.10008.1.2");
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(
            item.result,
            Some(PresentationContextResult::TransferSyntaxesNotSupported)
        );
    }

    #[test]
    fn test_deserialize_unknown_item_type_byte_returns_error() {
        // 0xFF is not a valid AssociateItemType; try_into() must reject it
        let mut data = ac_bytes(0x01, 0x00, "1.2.840.10008.1.2");
        data[0] = 0xFF;

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::UnrecognizedItemType(0xFF))
        ));
    }

    #[test]
    fn test_deserialize_non_presentation_context_item_type_returns_error() {
        // 0x50 (UserInformation) is a valid AssociateItemType but not allowed as a presentation
        // context item type; PresentationContextItem::new must reject it
        let mut data = ac_bytes(0x01, 0x00, "1.2.840.10008.1.2");
        data[0] = 0x50;

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::InvalidPresentationItem(
                PresentationContextError::InvalidItemType(_)
            ))
        ));
    }

    #[test]
    fn test_deserialize_empty_reader_returns_error() {
        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(vec![])),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_truncated_before_item_length_returns_error() {
        // item_type byte + padding = 2 bytes; the 2-byte item_length field cannot be read
        let data = vec![0x20_u8, 0x00];
        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_truncated_within_syntax_bytes_returns_error() {
        // Valid Rq header, but the last 5 bytes of the transfer syntax string are missing
        let mut data = rq_bytes(0x01, "1.2.840.10008.1.1", &["1.2.840.10008.1.2"]);
        data.truncate(data.len() - 5);

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_unknown_syntax_type_inside_context_returns_error() {
        // 0xFF inside the syntax sub-item area is an unknown type; the inner loop must propagate
        // the error rather than silently skipping it
        let syntax_len = 17u16; // "1.2.840.10008.1.1"
        let item_length = 4u16 + 4 + syntax_len; // context header + corrupted syntax item

        let mut data = vec![0x20, 0x00];
        data.extend_from_slice(&item_length.to_be_bytes());
        data.extend_from_slice(&[0x01, 0x00, 0xFF, 0x00]); // context_id, pad, no-result, pad
        data.push(0xFF); // unknown syntax item type
        data.push(0x00);
        data.extend_from_slice(&syntax_len.to_be_bytes());
        data.extend_from_slice(b"1.2.840.10008.1.1");

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::UnrecognizedItemType(0xFF))
        ));
    }

    #[test]
    fn test_deserialize_invalid_utf8_in_syntax_returns_error() {
        // Non-UTF-8 bytes inside an abstract syntax string must surface as InvalidEncoding
        let invalid_utf8: &[u8] = &[0xFF, 0xFE, 0xFD];
        let abs_len = invalid_utf8.len() as u16;
        let item_length = 4u16 + 4 + abs_len;

        let mut data = vec![0x20, 0x00];
        data.extend_from_slice(&item_length.to_be_bytes());
        data.extend_from_slice(&[0x01, 0x00, 0xFF, 0x00]);
        data.push(0x30); // AbstractSyntax
        data.push(0x00);
        data.extend_from_slice(&abs_len.to_be_bytes());
        data.extend_from_slice(invalid_utf8);

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::InvalidEncoding(_))
        ));
    }

    #[test]
    fn test_deserialize_recognized_non_syntax_item_type_in_variable_field_returns_error() {
        // 0x10 (ApplicationContext) is a recognized AssociateItemType but is not valid as a
        // syntax sub-item; the loop must return UnexpectedItemType rather than silently skipping
        let syntax_len = 17u16; // "1.2.840.10008.1.1"
        let item_length = 4u16 + 4 + syntax_len;

        let mut data = vec![0x20, 0x00];
        data.extend_from_slice(&item_length.to_be_bytes());
        data.extend_from_slice(&[0x01, 0x00, 0xFF, 0x00]);
        data.push(0x10); // ApplicationContext — recognized but unexpected as a syntax sub-item
        data.push(0x00);
        data.extend_from_slice(&syntax_len.to_be_bytes());
        data.extend_from_slice(b"1.2.840.10008.1.1");

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::UnexpectedItemType(
                AssociateItemType::ApplicationContext
            ))
        ));
    }

    #[test]
    fn test_deserialize_invalid_utf8_in_transfer_syntax_returns_error() {
        // Non-UTF-8 bytes inside a transfer syntax string must surface as InvalidEncoding
        let invalid_utf8: &[u8] = &[0xFF, 0xFE, 0xFD];
        let ts_len = invalid_utf8.len() as u16;
        let item_length = 4u16 + 4 + ts_len;

        let mut data = vec![0x21, 0x00]; // PresentationContextAc
        data.extend_from_slice(&item_length.to_be_bytes());
        data.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // context_id, pad, Acceptance, pad
        data.push(0x40); // TransferSyntax
        data.push(0x00);
        data.extend_from_slice(&ts_len.to_be_bytes());
        data.extend_from_slice(invalid_utf8);

        assert!(matches!(
            deserialize_presentation_context_item(&mut Cursor::new(data)),
            Err(PduDeserializationError::InvalidEncoding(_))
        ));
    }

    #[test]
    fn test_deserialize_rq_preserves_min_context_id() {
        let data = rq_bytes(0x00, "1.2.840.10008.1.1", &["1.2.840.10008.1.2"]);
        let item = deserialize_presentation_context_item(&mut Cursor::new(data)).unwrap();
        assert_eq!(item.context_id, 0x00);
    }

    #[test]
    fn test_deserialize_rq_roundtrip_with_serialize() {
        let original = PresentationContextItem::new(
            AssociateItemType::PresentationContextRq,
            0x01,
            None,
            Some(SyntaxItem::new(AssociateItemType::AbstractSyntax, "1.2.840.10008.1.1").unwrap()),
            vec![SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap()],
        )
        .unwrap();

        let serialized = serialize_presentation_context_item(&original);
        let deserialized =
            deserialize_presentation_context_item(&mut Cursor::new(serialized)).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_deserialize_ac_roundtrip_with_serialize() {
        let original = PresentationContextItem::new(
            AssociateItemType::PresentationContextAc,
            0x03,
            Some(PresentationContextResult::Acceptance),
            None,
            vec![SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap()],
        )
        .unwrap();

        let serialized = serialize_presentation_context_item(&original);
        let deserialized =
            deserialize_presentation_context_item(&mut Cursor::new(serialized)).unwrap();

        assert_eq!(original, deserialized);
    }

    fn abs() -> SyntaxItem {
        SyntaxItem::new(AssociateItemType::AbstractSyntax, "1.2.840.10008.1.1").unwrap()
    }

    fn ts() -> SyntaxItem {
        SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap()
    }

    fn ts2() -> SyntaxItem {
        SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2.1").unwrap()
    }

    #[test]
    fn test_builder_rq_produces_correct_item() {
        let item = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextRq)
            .context_id(0x01)
            .abstract_syntax_item(abs())
            .add_transfer_syntax(ts())
            .build()
            .unwrap();

        assert_eq!(item.item_type, AssociateItemType::PresentationContextRq);
        assert_eq!(item.context_id, 0x01);
        assert_eq!(item.result, None);
        assert_eq!(item.abstract_syntax(), Some("1.2.840.10008.1.1"));
        assert_eq!(item.transfer_syntax(), vec!["1.2.840.10008.1.2"]);
    }

    #[test]
    fn test_builder_ac_produces_correct_item() {
        let item = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextAc)
            .context_id(0x03)
            .result(PresentationContextResult::Acceptance)
            .add_transfer_syntax(ts())
            .build()
            .unwrap();

        assert_eq!(item.item_type, AssociateItemType::PresentationContextAc);
        assert_eq!(item.context_id, 0x03);
        assert_eq!(item.result, Some(PresentationContextResult::Acceptance));
        assert_eq!(item.abstract_syntax(), None);
        assert_eq!(item.transfer_syntax(), vec!["1.2.840.10008.1.2"]);
    }

    #[test]
    fn test_builder_add_transfer_syntax_accumulates() {
        let item = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextRq)
            .context_id(0x01)
            .abstract_syntax_item(abs())
            .add_transfer_syntax(ts())
            .add_transfer_syntax(ts2())
            .build()
            .unwrap();

        assert_eq!(
            item.transfer_syntax(),
            vec!["1.2.840.10008.1.2", "1.2.840.10008.1.2.1"]
        );
    }

    #[test]
    fn test_builder_transfer_syntax_items_replaces_accumulated() {
        let item = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextRq)
            .context_id(0x01)
            .abstract_syntax_item(abs())
            .add_transfer_syntax(ts())
            .transfer_syntax_items(vec![ts2()])
            .build()
            .unwrap();

        assert_eq!(item.transfer_syntax(), vec!["1.2.840.10008.1.2.1"]);
    }

    #[test]
    fn test_builder_invalid_item_type_returns_error() {
        let result = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::UserInformation)
            .context_id(0x01)
            .add_transfer_syntax(ts())
            .build();

        assert!(matches!(
            result,
            Err(PresentationContextError::InvalidItemType(
                AssociateItemType::UserInformation
            ))
        ));
    }

    #[test]
    fn test_builder_rq_length_matches_direct_construction() {
        let via_builder = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextRq)
            .context_id(0x01)
            .abstract_syntax_item(abs())
            .add_transfer_syntax(ts())
            .add_transfer_syntax(ts2())
            .build()
            .unwrap();

        let direct = PresentationContextItem::new(
            AssociateItemType::PresentationContextRq,
            0x01,
            None,
            Some(abs()),
            vec![ts(), ts2()],
        )
        .unwrap();

        assert_eq!(via_builder.length, direct.length);
        assert_eq!(via_builder.item_length(), direct.item_length());
    }

    #[test]
    fn test_builder_ac_all_result_variants() {
        for result in [
            PresentationContextResult::Acceptance,
            PresentationContextResult::UserRejection,
            PresentationContextResult::NoReason,
            PresentationContextResult::AbstractSyntaxNotSupported,
            PresentationContextResult::TransferSyntaxesNotSupported,
        ] {
            let item = PresentationContextItemBuilder::new()
                .item_type(AssociateItemType::PresentationContextAc)
                .context_id(0x01)
                .result(result)
                .add_transfer_syntax(ts())
                .build()
                .unwrap();

            assert_eq!(item.result, Some(result));
        }
    }

    #[test]
    fn test_builder_rq_equals_direct_construction() {
        let via_builder = PresentationContextItemBuilder::new()
            .item_type(AssociateItemType::PresentationContextRq)
            .context_id(0x05)
            .abstract_syntax_item(abs())
            .add_transfer_syntax(ts())
            .build()
            .unwrap();

        let direct = PresentationContextItem::new(
            AssociateItemType::PresentationContextRq,
            0x05,
            None,
            Some(abs()),
            vec![ts()],
        )
        .unwrap();

        assert_eq!(via_builder, direct);
    }
}
