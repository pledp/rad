use std::convert::TryFrom;
use std::io::Read;
use std::string::String;

use crate::{Result, Error};
use crate::associate::{ AssociateRequestAcceptPdu };

const APPLICATION_CONTEXT_NAME: &'static str = "1.2.840.10008.3.1.1.1";

// TODO: Implement for all PDU and related items
pub trait SerializableItem {
    fn serialize(&self) -> Result<Vec<u8>>;
}

#[repr(u8)]
pub enum PduType {
    AssociateRequest = 0x01,
    AssociateAccept = 0x02,
    AssociateReject = 0x03,
    Data = 0x04,
    ReleaseRequest = 0x05,
    ReleaseResponse = 0x06,
    Abort = 0x07,
}

impl TryFrom<u8> for PduType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(PduType::AssociateRequest),
            0x02 => Ok(PduType::AssociateAccept),
            0x03 => Ok(PduType::AssociateReject),
            0x04 => Ok(PduType::Data),
            0x05 => Ok(PduType::ReleaseRequest),
            0x06 => Ok(PduType::ReleaseResponse),
            0x07 => Ok(PduType::Abort),
            _ => Err("Invalid type".into()),
        }
    }
}

pub struct PduHeader {
    pub pdu_type: PduType,
    pub length: u32,
}

pub fn read_pdu_header<R: Read>(reader: &mut R) -> Result<PduHeader> {
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf)?;

    read_padding(reader, 1);

    let mut length_buf = [0u8; 4];
    reader.read_exact(&mut length_buf)?;

    Ok(PduHeader {
        pdu_type: type_buf[0].try_into()?,
        length: u32::from_be_bytes(length_buf),
    })
}

// TODO: Add validation that read padding is actually 0x00
pub(crate) fn read_padding<R: Read>(reader: &mut R, n: usize) {
    let mut buf = vec!(0u8; n);
    reader.read_exact(&mut buf).unwrap();
}

pub(crate) fn vec8_add_padding(pdu: &mut Vec<u8>, n: u32) {
    pdu.extend(std::iter::repeat(0x00).take(n as usize));
}
