use std::io::{BufRead, BufReader, Read};

use crate::ul::associate::AssociateItemType;
use crate::ul::associate::ITEM_LENGTH_LENGTH;
use crate::ul::associate::PduDeserializationError;
use crate::pdu::{PDU_TYPE_LENGTH, read_padding, vec8_add_padding};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UserInformation {
    MaximumLength(MaximumLength),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaximumLength {
    pub maximum_length: u32,
}

#[derive(Debug, PartialEq)]
pub struct UserInfoItem {
    pub item_type: AssociateItemType,
    pub length: u16,
    pub sub_items: Vec<UserInformationSubItem>,
}

impl UserInfoItem {
    pub fn new(sub_items: Vec<UserInformationSubItem>) -> Self {
        let mut length = 0;

        length += sub_items.iter().map(|item| item.item_length()).sum::<u32>();

        Self {
            item_type: AssociateItemType::UserInformation,
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

pub struct UserInfoItemBuilder {
    sub_items: Vec<UserInformationSubItem>,
}

impl UserInfoItemBuilder {
    pub fn new() -> Self {
        Self {
            sub_items: Vec::new(),
        }
    }

    pub fn add_sub_item(mut self, item: UserInformationSubItem) -> Self {
        self.sub_items.push(item);
        self
    }

    pub fn sub_items(mut self, items: Vec<UserInformationSubItem>) -> Self {
        self.sub_items = items;
        self
    }

    pub fn build(self) -> UserInfoItem {
        UserInfoItem::new(self.sub_items)
    }
}

pub fn serialize_user_info_item(item: &UserInfoItem) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&item.length.to_be_bytes());

    for item in item.sub_items.iter() {
        pdu.extend(serialize_sub_item(item));
    }

    pdu
}

pub fn deserialize_user_info_item<T: Read>(reader: &mut T) -> Result<UserInfoItem, PduDeserializationError> {
    let mut pdu_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut pdu_type)?;

    let item_type: AssociateItemType = pdu_type[0].try_into()?;
    if item_type != AssociateItemType::UserInformation {
        return Err(PduDeserializationError::UnexpectedItemType(item_type));
    }

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    // Split reader into subreader which is expected to contain the rest of the contents presentation context item contents.
    let mut sub_item_reader = BufReader::new(reader.take(u16::from_be_bytes(item_length).into()));

    let mut sub_items: Vec<UserInformationSubItem> = Vec::new();

    while !sub_item_reader.fill_buf()?.is_empty() {
        sub_items.push(deserialize_sub_item(&mut sub_item_reader)?);
    }

    Ok(UserInfoItem::new(sub_items))
}

#[derive(Debug, PartialEq)]
pub struct UserInformationSubItem {
    pub item_type: u8,
    pub length: u16,
    pub inner: UserInformation,
}

impl UserInformationSubItem {
    pub fn new(inner: UserInformation) -> Self {
        Self {
            item_type: match inner {
                UserInformation::MaximumLength(_) => 0x51,
                _ => todo!(),
            },
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

/// Serializes a [UserInformationSubItem] into a [Vec<u8>].
pub fn serialize_sub_item(item: &UserInformationSubItem) -> Vec<u8> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());

    match &item.inner {
        UserInformation::MaximumLength(user_item) => {
            pdu.extend_from_slice(&user_item.maximum_length.to_be_bytes());
        }
    }

    pdu
}

/// Deserializes bytes from a [Read] into a [UserInformationSubItem].
///
/// # Errors
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/user_info_sub_item_deserialize_errors.md"))]
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/errors/deserialize_errors.md"))]
pub fn deserialize_sub_item<T: Read>(reader: &mut T) -> Result<UserInformationSubItem, PduDeserializationError> {
    let mut item_type = [0u8; PDU_TYPE_LENGTH];
    reader.read_exact(&mut item_type)?;
    let item_type: AssociateItemType = item_type[0].try_into()?;

    read_padding(reader, 1);

    let mut item_length = [0u8; ITEM_LENGTH_LENGTH];
    reader.read_exact(&mut item_length)?;

    let length = u16::from_be_bytes(item_length);

    let mut value = vec![0u8; length as usize];
    reader.read_exact(&mut value)?;

    Ok(UserInformationSubItem {
        item_type: item_type.into(),
        length,
        inner: match item_type {
            AssociateItemType::MaximumLength => {
                let arr: [u8; 4] = value[..4]
                    .try_into()
                    .expect("slice must be exactly 4 bytes");
                let maximum_length = u32::from_be_bytes(arr);
                UserInformation::MaximumLength(MaximumLength { maximum_length })
            }
            _ => return Err(PduDeserializationError::UnexpectedItemType(item_type)),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn test_serialize_sub_item_maximum_length_ok() {
        let item = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: 16384,
        }));

        assert_eq!(
            serialize_sub_item(&item),
            vec![0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00]
        );
    }

    #[test]
    fn test_serialize_sub_item_maximum_length_zero() {
        let item = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: 0,
        }));

        assert_eq!(
            serialize_sub_item(&item),
            vec![0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_serialize_sub_item_maximum_length_max() {
        let item = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: u32::MAX,
        }));

        assert_eq!(
            serialize_sub_item(&item),
            vec![0x51, 0x00, 0x00, 0x04, 0xFF, 0xFF, 0xFF, 0xFF]
        );
    }

    #[test]
    fn test_serialize_sub_item_round_trip() {
        let item = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: 16384,
        }));

        let serialized = serialize_sub_item(&item);
        let deserialized = deserialize_sub_item(&mut std::io::Cursor::new(serialized)).unwrap();

        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_deserialize_sub_item_maximum_length_ok() {
        let mut data = Cursor::new(vec![
            0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
        ]);

        let expected = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: 16384,
        }));

        assert_eq!(expected, deserialize_sub_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_sub_item_maximum_length_zero() {
        let mut data = Cursor::new(vec![
            0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
        ]);

        let expected = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: 0,
        }));

        assert_eq!(expected, deserialize_sub_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_sub_item_maximum_length_max() {
        let mut data = Cursor::new(vec![
            0x51, 0x00, 0x00, 0x04, 0xFF, 0xFF, 0xFF, 0xFF,
        ]);

        let expected = UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
            maximum_length: u32::MAX,
        }));

        assert_eq!(expected, deserialize_sub_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_sub_item_truncated_at_item_type() {
        let mut data = Cursor::new(vec![]);

        assert!(matches!(
            deserialize_sub_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_sub_item_truncated_at_item_length() {
        // item_type + padding only, no room for item_length field
        let mut data = Cursor::new(vec![0x51, 0x00]);

        assert!(matches!(
            deserialize_sub_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_sub_item_truncated_at_value() {
        // item_length declares 4 bytes but only 3 are present
        let mut data = Cursor::new(vec![0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40]);

        assert!(matches!(
            deserialize_sub_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_sub_item_unrecognized_item_type() {
        let mut data = Cursor::new(vec![
            0x80, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
        ]);

        assert!(matches!(
            deserialize_sub_item(&mut data),
            Err(PduDeserializationError::UnrecognizedItemType(_))
        ));
    }

    #[test]
    fn test_deserialize_sub_item_unexpected_item_type() {
        let cases = [
            (0x10, AssociateItemType::ApplicationContext),
            (0x40, AssociateItemType::TransferSyntax),
            (0x50, AssociateItemType::UserInformation),
        ];

        for (byte, expected_type) in cases {
            let mut data = Cursor::new(vec![byte, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00]);

            assert!(matches!(
                deserialize_sub_item(&mut data),
                Err(PduDeserializationError::UnexpectedItemType(t)) if t == expected_type
            ));
        }
    }

    #[test]
    fn test_serialize_user_info_item_ok() {
        let item = UserInfoItem::new(vec![UserInformationSubItem::new(
            UserInformation::MaximumLength(MaximumLength { maximum_length: 16384 }),
        )]);

        assert_eq!(
            serialize_user_info_item(&item),
            vec![
                0x50, 0x00, 0x00, 0x08, // UserInformation type, padding, length=8
                0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00, // MaximumLength(16384)
            ]
        );
    }

    #[test]
    fn test_serialize_user_info_item_no_sub_items() {
        let item = UserInfoItem::new(vec![]);

        assert_eq!(
            serialize_user_info_item(&item),
            vec![0x50, 0x00, 0x00, 0x00]
        );
    }

    #[test]
    fn test_serialize_user_info_item_multiple_sub_items() {
        let item = UserInfoItem::new(vec![
            UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
                maximum_length: 16384,
            })),
            UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
                maximum_length: u32::MAX,
            })),
        ]);

        assert_eq!(
            serialize_user_info_item(&item),
            vec![
                0x50, 0x00, 0x00, 0x10, // length=16 (two sub-items at 8 bytes each)
                0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
                0x51, 0x00, 0x00, 0x04, 0xFF, 0xFF, 0xFF, 0xFF,
            ]
        );
    }

    #[test]
    fn test_serialize_user_info_item_round_trip() {
        let item = UserInfoItem::new(vec![UserInformationSubItem::new(
            UserInformation::MaximumLength(MaximumLength { maximum_length: 16384 }),
        )]);

        let serialized = serialize_user_info_item(&item);
        let deserialized =
            deserialize_user_info_item(&mut Cursor::new(serialized)).unwrap();

        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_deserialize_user_info_item_ok() {
        let mut data = Cursor::new(vec![
            0x50, 0x00, 0x00, 0x08, // UserInformation type, padding, length=8
            0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00, // MaximumLength(16384)
        ]);

        let expected = UserInfoItem::new(vec![UserInformationSubItem::new(
            UserInformation::MaximumLength(MaximumLength { maximum_length: 16384 }),
        )]);

        assert_eq!(expected, deserialize_user_info_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_user_info_item_no_sub_items() {
        let mut data = Cursor::new(vec![0x50, 0x00, 0x00, 0x00]);

        assert_eq!(
            UserInfoItem::new(vec![]),
            deserialize_user_info_item(&mut data).unwrap()
        );
    }

    #[test]
    fn test_deserialize_user_info_item_multiple_sub_items() {
        let mut data = Cursor::new(vec![
            0x50, 0x00, 0x00, 0x10,
            0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
            0x51, 0x00, 0x00, 0x04, 0xFF, 0xFF, 0xFF, 0xFF,
        ]);

        let expected = UserInfoItem::new(vec![
            UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
                maximum_length: 16384,
            })),
            UserInformationSubItem::new(UserInformation::MaximumLength(MaximumLength {
                maximum_length: u32::MAX,
            })),
        ]);

        assert_eq!(expected, deserialize_user_info_item(&mut data).unwrap());
    }

    #[test]
    fn test_deserialize_user_info_item_truncated_at_item_type() {
        let mut data = Cursor::new(vec![]);

        assert!(matches!(
            deserialize_user_info_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_user_info_item_truncated_at_item_length() {
        // item_type + padding only, no room for item_length field
        let mut data = Cursor::new(vec![0x50, 0x00]);

        assert!(matches!(
            deserialize_user_info_item(&mut data),
            Err(PduDeserializationError::InvalidLength(_))
        ));
    }

    #[test]
    fn test_deserialize_user_info_item_unrecognized_item_type() {
        let mut data = Cursor::new(vec![
            0x80, 0x00, 0x00, 0x08,
            0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
        ]);

        assert!(matches!(
            deserialize_user_info_item(&mut data),
            Err(PduDeserializationError::UnrecognizedItemType(_))
        ));
    }

    #[test]
    fn test_deserialize_user_info_item_unexpected_item_type() {
        let cases = [
            (0x10, AssociateItemType::ApplicationContext),
            (0x40, AssociateItemType::TransferSyntax),
            (0x51, AssociateItemType::MaximumLength),
        ];

        for (byte, expected_type) in cases {
            let mut data = Cursor::new(vec![
                byte, 0x00, 0x00, 0x08,
                0x51, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
            ]);

            assert!(matches!(
                deserialize_user_info_item(&mut data),
                Err(PduDeserializationError::UnexpectedItemType(t)) if t == expected_type
            ));
        }
    }

    #[test]
    fn test_deserialize_user_info_item_invalid_sub_item_type() {
        // Outer header valid, but sub-item body contains an unrecognized type byte
        let mut data = Cursor::new(vec![
            0x50, 0x00, 0x00, 0x08,
            0x80, 0x00, 0x00, 0x04, 0x00, 0x00, 0x40, 0x00,
        ]);

        assert!(matches!(
            deserialize_user_info_item(&mut data),
            Err(PduDeserializationError::UnrecognizedItemType(_))
        ));
    }
}
