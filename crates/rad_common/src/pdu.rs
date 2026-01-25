use std::convert::TryFrom;
use std::io::Read;
use std::string::String;

use crate::Result;

#[repr(u8)]
enum Type {
    AssociateRq = 0x01,
    AssociateAc = 0x02,
    AssociateRj = 0x03,
    Data = 0x04,
    ReleaseRq = 0x05,
    ReleaseRp = 0x06,
    Abort = 0x07,
}

impl TryFrom<u8> for Type {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x01 => Ok(Type::AssociateRq),
            0x02 => Ok(Type::AssociateAc),
            0x03 => Ok(Type::AssociateRj),
            0x04 => Ok(Type::Data),
            0x05 => Ok(Type::ReleaseRq),
            0x06 => Ok(Type::ReleaseRp),
            0x07 => Ok(Type::Abort),
            _ => Err("Invalid type".into()),
        }
    }
}

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

/*
enum State {
    Idle,
    _.
    _.
    AwaitTransportConnection,
    AwaitAcRjPdu
}

*/

pub enum DicomPdu {
    AssociateRqAc(AssociatePdu),
    Data(AssociatePdu),
}

pub struct AssociatePdu {
    pub pdu_type: Type,
    pub length: u32,
    pub called_ae: String,
    pub calling_ae: String,
}

pub fn read_dicom_pdu<R: Read>(mut reader: R) -> Result<DicomPdu> {
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf);

    let pdu_type: Type = type_buf[0].try_into()?;

    Ok(DicomPdu::AssociateRqAc(AssociatePdu {
        pdu_type: Type::AssociateRq,
        length: 2,
        called_ae: "test".into(),
        calling_ae: "test".into(),
    }))
}

pub fn read_associate_rq_ac<R: Read>(mut reader: R) -> Result<AssociatePdu> {
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf);

    let pdu_type: Type = type_buf[0].try_into()?;

    Ok(AssociatePdu {
        pdu_type: Type::AssociateRq,
        length: 2,
        called_ae: "test".into(),
        calling_ae: "test".into(),
    })
}
