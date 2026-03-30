use crate::{associate::AssociationItemType, Result};

/// Length of the presentation context item without the variable field.
pub const PRESENTATION_CONTEXT_ITEM_NO_VARIABLE_FIELDS_LENGTH: u16 = 4;

pub struct PresentationContextItem {
    pub item_type: AssociationItemType,
    pub length: u16,
    pub context_id: u8,
    pub result: Option<AssociateResult>,
    pub abstract_syntax_item: Option<SyntaxItem>,
    pub transfer_syntax_items: Vec<SyntaxItem>,
}

impl PresentationContextItem {
    pub fn new(item_type: AssociationItemType, context_id: u8, result: Option<AssociateResult>, abstract_syntax_item: Option<SyntaxItem>, transfer_syntax_items: Vec<SyntaxItem>) -> Result<Self> {
        match item_type {
            AssociationItemType::PresentationContextRq => {
                Ok(PresentationContextItem::new_rq(context_id, abstract_syntax_item.unwrap(), transfer_syntax_items))
            }
            AssociationItemType::PresentationContextAc => {
                todo!()
            }
            _ => {
                return Err("Invalid type".into())
            }
        }
    }

    fn new_rq(context_id: u8, abstract_syntax_item: SyntaxItem, transfer_syntax_items: Vec<SyntaxItem>) -> Self {
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

    fn new_ac(result: AssociateResult) -> Self {
        todo!();
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

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum AssociateResult {
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

pub struct SyntaxItem {
    pub item_type: AssociationItemType,
    pub length: u16,
    syntax: String,
}

impl SyntaxItem {
    pub fn new(item_type: AssociationItemType, syntax: &str) -> Self {
        Self {
            item_type,
            length: syntax.len() as u16,
            syntax: syntax.into(),
        }
    }

    pub fn item_length(&self) -> u32 {
        const SYNTAX_ITEM_DEFAULT_LENGTH: u32 = 4;
        println!(
            "LENGTH: {}",
            SYNTAX_ITEM_DEFAULT_LENGTH + self.length as u32
        );

        SYNTAX_ITEM_DEFAULT_LENGTH + self.length as u32
    }

    pub fn syntax(&self) -> &str {
        &self.syntax
    }
}

pub struct PresentationContextItemBuilder {
    item_type: Option<AssociationItemType>,
    context_id: Option<u8>,
    result: Option<AssociateResult>,
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

    pub fn result(mut self, result: AssociateResult) -> Self {
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

    pub fn build(self) -> Result<PresentationContextItem> {
        Ok(PresentationContextItem::new(self.item_type.unwrap(), self.context_id.unwrap(), self.result, self.abstract_syntax_item, self.transfer_syntax_items)?)
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

    pub fn build(self) -> Result<SyntaxItem> {
        Ok(SyntaxItem::new(self.item_type.unwrap(), &self.syntax.unwrap()))
    }
}
