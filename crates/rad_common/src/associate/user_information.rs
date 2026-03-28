use crate::Result;
use crate::associate::UserInformationSubItem;

#[derive(Clone, Copy)]
pub enum UserInformation {
    MaximumLength(MaximumLength),
}

#[derive(Clone, Copy)]
pub struct MaximumLength {
    pub maximum_length: u32,
}
