use std::convert::TryFrom;
use std::io::Read;
use std::string::String;

use crate::associate::AAssociateRqAc;
use crate::Result;

const APPLICATION_CONTEXT_NAME: &'static str = "1.2.840.10008.3.1.1.1";

#[repr(u8)]
pub enum PduType {
    AssociateRq = 0x01,
    AssociateAc = 0x02,
    AssociateRj = 0x03,
    Data = 0x04,
    ReleaseRq = 0x05,
    ReleaseRp = 0x06,
    Abort = 0x07,
}

impl TryFrom<u8> for PduType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(PduType::AssociateRq),
            0x02 => Ok(PduType::AssociateAc),
            0x03 => Ok(PduType::AssociateRj),
            0x04 => Ok(PduType::Data),
            0x05 => Ok(PduType::ReleaseRq),
            0x06 => Ok(PduType::ReleaseRp),
            0x07 => Ok(PduType::Abort),
            _ => Err("Invalid type".into()),
        }
    }
}

pub enum DicomPdu {
    AssociateRqAc(AAssociateRqAc),
    Data(AAssociateRqAc),
}

pub fn read_dicom_pdu<R: Read>(mut reader: R) -> Result<DicomPdu> {
    todo!();
    /*
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf);

    let pdu_type: PduType = type_buf[0].try_into()?;

    // TODO: Switch
    Ok(DicomPdu::AssociateRqAc(AAssociateRq {
        pdu_type: PduType::AssociateRq,
        length: 2,
        called_ae: "test".into(),
        calling_ae: "test".into(),

    */
}

pub(crate) fn vec8_add_padding(pdu: &mut Vec<u8>, n: u32) {
    pdu.extend(std::iter::repeat(0x00).take(n as usize));
}
