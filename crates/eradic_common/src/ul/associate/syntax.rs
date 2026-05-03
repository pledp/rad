use std::io::Read;

use thiserror::Error;

use crate::ul::{
    associate::{AssociateItemType, ITEM_LENGTH_LENGTH, PduDeserializationError},
    pdu::{PDU_TYPE_LENGTH, read_padding, vec8_add_padding},
};

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
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/syntax_deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/item_deserialize_errors.md"))]
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
        item_type[0].try_into()?,
        &String::from_utf8(syntax)?,
    )?)
}

#[derive(Debug, Error)]
pub enum SyntaxItemError {
    #[error(
        "Item type must be {:?} or {:?}",
        AssociateItemType::AbstractSyntax,
        AssociateItemType::TransferSyntax
    )]
    InvalidItemType,
}

#[derive(Debug, PartialEq)]
pub struct SyntaxItem {
    pub item_type: AssociateItemType,
    pub length: u16,
    syntax: String,
}

impl SyntaxItem {
    /// Creates a SyntaxItem.
    ///
    /// [SyntaxItem] may represent an abstract syntax or a transfer syntax.
    ///
    /// # Errors
    /// [`SyntaxItemError::InvalidItemType`] if `item_type` is not of [`AssociateItemType::AbstractSyntax`] or [`AssociateItemType::TransferSyntax`].
    pub fn new(item_type: AssociateItemType, syntax: &str) -> Result<Self, SyntaxItemError> {
        match item_type {
            AssociateItemType::AbstractSyntax | AssociateItemType::TransferSyntax => {}
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
    item_type: Option<AssociateItemType>,
    syntax: Option<String>,
}

impl SyntaxItemBuilder {
    pub fn new() -> Self {
        Self {
            item_type: None,
            syntax: None,
        }
    }

    pub fn item_type(mut self, item_type: AssociateItemType) -> Self {
        self.item_type = Some(item_type);
        self
    }

    pub fn syntax<S: Into<String>>(mut self, syntax: S) -> Self {
        self.syntax = Some(syntax.into());
        self
    }

    pub fn build(self) -> Result<SyntaxItem, SyntaxItemError> {
        SyntaxItem::new(self.item_type.unwrap(), &self.syntax.unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_syntax_item_new_ok() {
        assert!(SyntaxItem::new(AssociateItemType::AbstractSyntax, "1.2.840.10008.1.1").is_ok());
        assert!(SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").is_ok());
    }

    #[test]
    fn test_syntax_item_length() {
        let mut data = vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ];

        let item = SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.1").unwrap();
        assert_eq!(item.item_length(), data.len() as u32);
    }

    #[test]
    fn test_syntax_item_new_err() {
        assert!(matches!(
            SyntaxItem::new(
                AssociateItemType::PresentationContextAc,
                "1.2.840.10008.1.1"
            ),
            Err(SyntaxItemError::InvalidItemType)
        ));
        assert!(matches!(
            SyntaxItem::new(AssociateItemType::UserInformation, "1.2.840.10008.1.1"),
            Err(SyntaxItemError::InvalidItemType)
        ));
    }

    #[test]
    fn test_deserialize_syntax_item_ok() {
        let mut data = Cursor::new(vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ]);

        let item = SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap();

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
    fn test_deserialize_syntax_item_encode_err() {
        let mut data = Cursor::new(vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x82, 0x32, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ]);

        assert!(matches!(
            deserialize_syntax_item(&mut data),
            Err(PduDeserializationError::InvalidEncoding(_))
        ));
    }

    #[test]
    fn test_serialize_syntax_item_ok() {
        let data = vec![
            0x40, 0x00, 0x00, 0x11, 0x31, 0x2e, 0x32, 0x2e, 0x38, 0x34, 0x30, 0x2e, 0x31, 0x30,
            0x30, 0x30, 0x38, 0x2e, 0x31, 0x2e, 0x32,
        ];

        assert_eq!(
            serialize_syntax_item(
                &SyntaxItem::new(AssociateItemType::TransferSyntax, "1.2.840.10008.1.2").unwrap()
            ),
            data
        );
    }
}
