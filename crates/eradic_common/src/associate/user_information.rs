use std::io::{BufRead, BufReader, Read};

use crate::Result;
use crate::associate::AssociateItemType;
use crate::associate::ITEM_LENGTH_LENGTH;
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

    pub fn build(self) -> Result<UserInfoItem> {
        Ok(UserInfoItem::new(self.sub_items))
    }
}

pub fn serialize_user_info_item(item: &UserInfoItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type.into());
    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&item.length.to_be_bytes());

    for item in item.sub_items.iter() {
        pdu.extend(serialize_sub_item(item)?);
    }

    Ok(pdu)
}

pub fn deserialize_user_info_item<T: Read>(reader: &mut T) -> Result<UserInfoItem> {
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

pub fn serialize_sub_item(item: &UserInformationSubItem) -> Result<Vec<u8>> {
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

pub fn deserialize_sub_item<T: Read>(reader: &mut T) -> Result<UserInformationSubItem> {
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
