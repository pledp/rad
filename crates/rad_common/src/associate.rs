use std::convert::TryFrom;
use std::io::Read;
use std::string::String;

use crate::pdu::{ DicomPdu, PduType, vec8_add_padding };
use crate::Result;

const APPLICATION_CONTEXT_NAME: &'static str = "1.2.840.10008.3.1.1.1";

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

#[derive(Clone)]
enum AssociateResult {
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

// TODO: item_type struct
struct ApplicationContextItem {
    pub item_type: u8,
    pub length: u16,
    pub context_name: String,
}

struct PresentationContextItem {
    pub item_type: u8,
    pub length: u16,
    pub context_id: u8,
    pub result: Option<AssociateResult>,
    // pub abstract_syntax_item: Option<AbstractSyntaxItem>,
    // pub transfer_syntax_item: Vec<TransferSyntaxItem>,
}

pub struct AAssociateRq {
    pub pdu_type: PduType,
    pub length: u32,
    pub protocol_version: u16,
    pub called_ae: String,
    pub calling_ae: String,
    pub application_context_item: ApplicationContextItem,
    pub presentation_context_items: Vec<PresentationContextItem>,
    //pub user_info: UserInfoItem,
}

pub fn read_associate_rq_ac<R: Read>(mut reader: R) -> Result<AAssociateRq> {
    let mut type_buf = [0u8; 1];
    reader.read_exact(&mut type_buf);

    let pdu_type: PduType = type_buf[0].try_into()?;

    Ok(AAssociateRq {
        pdu_type: PduType::AssociateRq,
        length: 2,
        called_ae: "test".into(),
        calling_ae: "test".into(),
    })
}

pub fn serialize_association_rq(request: &AAssociateRq) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(0x01);

    vec8_add_padding(&mut pdu, 1);

    pdu.extend_from_slice(&request.length.to_be_bytes());
    pdu.extend_from_slice(&request.protocol_version.to_be_bytes());

    vec8_add_padding(&mut pdu, 2);

    pdu.extend_from_slice(&request.called_ae.as_bytes());

    vec8_add_padding(&mut pdu, 32);

    pdu.extend(
        serialize_application_context_item(&request.application_context_item)?
    );

    for item in request.presentation_context_items.iter() {
        pdu.extend(serialize_presentation_context_item(item)?);
    }

    Ok(pdu)
}

fn serialize_application_context_item(item: &ApplicationContextItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(0x10);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.extend_from_slice(&item.context_name.as_bytes());

    Ok(pdu)
}

fn serialize_presentation_context_item(item: &PresentationContextItem) -> Result<Vec<u8>> {
    let mut pdu: Vec<u8> = Vec::new();

    pdu.push(item.item_type);
    vec8_add_padding(&mut pdu, 1);
    pdu.extend_from_slice(&item.length.to_be_bytes());
    pdu.push(item.context_id);

    // Add result if it exists
    if let Some(result) = item.result.clone() {
        pdu.push(result.into());
    }
    else {
        vec8_add_padding(&mut pdu, 1);
    }

    vec8_add_padding(&mut pdu, 1);

    Ok(pdu)
}
