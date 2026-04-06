use std::net::IpAddr;

use crate::Result;
use crate::associate::RejectedAssociationResult;
use crate::associate::presentation_context::SyntaxItem;
use crate::associate::rj::ServiceUserReason;
use crate::associate::{
    AssociateRqAcPdu, MaximumLength, UserInformation, UserInformationSubItem,
    presentation_context::PresentationContextItem,
};

/// DICOM ISO/TR 8509 request and indication primitive. Request and indication contain the same fields.
///
/// Indicates a request to establish an association or a request PDU via TCP.
#[derive(Clone)]
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
    pub fn new(
        context_name: String,
        called_ae: String,
        calling_ae: String,
        user_information: Vec<UserInformation>,
        called_address: IpAddr,
        calling_address: IpAddr,
        presentation_context: Vec<PresentationContextDefinitionList>,
    ) -> Self {
        Self {
            context_name,
            called_ae,
            calling_ae,
            user_information,
            called_address,
            calling_address,
            presentation_context,
        }
    }

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

    pub fn presentation_context(&self) -> &Vec<PresentationContextDefinitionList> {
        &self.presentation_context
    }

    pub fn user_information(&self) -> &Vec<UserInformation> {
        &self.user_information
    }
}

#[derive(Clone)]
pub struct PresentationContextDefinitionList {
    pub context_id: u8,
    pub abstract_syntax: Option<String>,
    pub transfer_syntax: Vec<String>,
}

impl PresentationContextDefinitionList {
    pub fn new(
        context_id: u8,
        abstract_syntax: Option<String>,
        transfer_syntax: Vec<String>,
    ) -> Self {
        Self {
            context_id,
            abstract_syntax,
            transfer_syntax,
        }
    }

    /// Create [PresentationContextDefinitionList] from [PresentationContextItem]
    pub fn from_presentation_context_item(item: &PresentationContextItem) -> Self {
        Self {
            context_id: item.context_id,
            abstract_syntax: Some(item.abstract_syntax().unwrap().to_string()),
            transfer_syntax: item
                .transfer_syntax()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

pub struct PresentationContextDefinitionListBuilder {
    context_id: Option<u8>,
    abstract_syntax_item: Option<String>,
    transfer_syntax_items: Vec<String>,
}

impl PresentationContextDefinitionListBuilder {
    pub fn new() -> Self {
        Self {
            context_id: None,
            abstract_syntax_item: None,
            transfer_syntax_items: Vec::new(),
        }
    }

    pub fn context_id(mut self, context_id: u8) -> Self {
        self.context_id = Some(context_id);
        self
    }

    pub fn abstract_syntax(mut self, item: String) -> Self {
        self.abstract_syntax_item = Some(item);
        self
    }

    pub fn add_transfer_syntax(mut self, item: String) -> Self {
        self.transfer_syntax_items.push(item);
        self
    }

    pub fn transfer_syntax(mut self, items: Vec<String>) -> Self {
        self.transfer_syntax_items = items;
        self
    }

    pub fn build(self) -> Result<PresentationContextDefinitionList> {
        Ok(PresentationContextDefinitionList::new(
            self.context_id.unwrap(),
            self.abstract_syntax_item,
            self.transfer_syntax_items,
        ))
    }
}

pub enum AssociateRequestResponse {
    Accepted(AcceptedAssociateRequestResponse),
    Rejected(RejectedAssociateRequestResponse),
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
    pub result: RejectedAssociationResult,
}

impl RejectedAssociateRequestResponse {
    pub fn new(diagnostic: Option<ServiceUserReason>, result: RejectedAssociationResult) -> Self {
        Self { diagnostic, result }
    }
}
