use std::net::IpAddr;

use crate::Result;
use crate::associate::RejectedAssociationResult;
use crate::associate::rj::ServiceUserReason;
use crate::associate::{
    AssociateRqAcPdu, MaximumLength, presentation_context::PresentationContextItem, UserInformation,
    UserInformationSubItem,
};

pub struct AssociateRequestIndication {
    pub context_name: String,
    pub called_ae: String,
    pub calling_ae: String,
    pub user_information: Vec<UserInformation>,
    pub called_address: IpAddr,
    pub calling_address: IpAddr,
    pub presentation_context: Vec<PresentationContextDefinitionList>,
}

impl AssociateRequestIndication {
    pub fn from_rq_pdu(
        pdu: AssociateRqAcPdu,
        called_address: &IpAddr,
        calling_address: &IpAddr,
    ) -> Self {
        // Create Vector of [PresentationContextDefinitionList]
        let presentation_context = pdu
            .presentation_context_items()
            .iter()
            .map(|item| PresentationContextDefinitionList::from_presentation_context_item(item))
            .collect();

        let user_information = pdu
            .user_information()
            .iter()
            .map(|item| item.inner().clone())
            .collect();

        Self {
            context_name: pdu.context_name().to_string(),
            called_ae: pdu.called_ae().to_string(),
            calling_ae: pdu.calling_ae().to_string(),
            user_information,
            called_address: called_address.clone(),
            calling_address: calling_address.clone(),
            presentation_context,
        }
    }
}

pub struct PresentationContextDefinitionList {
    pub context_id: u8,
    pub abstract_syntax: String,
    pub transfer_syntax: Vec<String>,
}

impl PresentationContextDefinitionList {
    /// Create [PresentationContextDefinitionList] from [PresentationContextItem]
    pub fn from_presentation_context_item(item: &PresentationContextItem) -> Self {
        Self {
            context_id: item.context_id,
            abstract_syntax: item.abstract_syntax().unwrap().to_string(),
            transfer_syntax: item
                .transfer_syntax()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

pub enum AssociateRequestResponse {
    Accepted(AcceptedAssociateRequestResponse),
    Rejected(RejectedAssociateRequestResponse)
}

pub struct AcceptedAssociateRequestResponse {
    pub context_name: String,
    pub called_ae: String,
    pub calling_ae: String,
    pub user_information: Vec<UserInformation>,
    pub presentation_context_result: Vec<PresentationContextDefinitionList>,
}

pub struct RejectedAssociateRequestResponse {
    pub diagnostic: Option<ServiceUserReason>,
    pub result: RejectedAssociationResult
}

impl RejectedAssociateRequestResponse {
    pub fn new(
        diagnostic: Option<ServiceUserReason>,
        result: RejectedAssociationResult,
    ) -> Self {
        Self { diagnostic, result }
    }
}
