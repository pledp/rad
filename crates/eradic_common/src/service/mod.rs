use std::net::IpAddr;
use std::ops::Not;
use std::result;

use thiserror::Error;

use crate::associate::RejectedAssociationResult;
use crate::associate::presentation_context::{PresentationContextResult, SyntaxItem};
use crate::associate::rj::ServiceUserReason;
use crate::associate::{
    AssociateRqAcPdu, MaximumLength, UserInformation, UserInformationSubItem,
    presentation_context::PresentationContextItem,
};

/// DICOM ISO/TR 8509 request and indication primitive. Request and indication contain the same fields.
///
/// Indicates a request to establish an association or a request PDU via TCP.
#[derive(Clone, Debug)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationContextDefinitionList {
    pub context_id: u8,
    pub abstract_syntax: String,
    pub transfer_syntax: Vec<String>,
}

impl PresentationContextDefinitionList {
    pub fn new(context_id: u8, abstract_syntax: String, transfer_syntax: Vec<String>) -> Self {
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
            abstract_syntax: item.abstract_syntax().unwrap().to_string(),
            transfer_syntax: item
                .transfer_syntax()
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PresentationContextDefinitionResultList {
    pub context_id: u8,
    pub abstract_syntax: String,
    pub transfer_syntax: Vec<String>,
    pub result: PresentationContextResult,
}

impl PresentationContextDefinitionResultList {
    pub fn new(
        context_id: u8,
        abstract_syntax: String,
        transfer_syntax: Vec<String>,
        result: PresentationContextResult,
    ) -> Self {
        Self {
            context_id,
            abstract_syntax,
            transfer_syntax,
            result,
        }
    }

    pub fn from_definition_list(
        list: PresentationContextDefinitionList,
        result: PresentationContextResult,
    ) -> Self {
        Self {
            context_id: list.context_id,
            abstract_syntax: list.abstract_syntax,
            transfer_syntax: list.transfer_syntax,
            result,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PresentationContextDefinitionListBuilderError {
    #[error("PresentationContextDefinitionList must have atleast one trasnfer syntax")]
    NoTransferSyntax,
    #[error("PresentationContextDefinitionList must have atleast one trasnfer syntax")]
    NoAbstractSyntax,
    #[error("PresentationContextDefinitionList must have a context_id")]
    NoContextId,
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

    pub fn build(
        self,
    ) -> Result<PresentationContextDefinitionList, PresentationContextDefinitionListBuilderError>
    {
        if self.transfer_syntax_items.is_empty() {
            return Err(PresentationContextDefinitionListBuilderError::NoTransferSyntax);
        }

        Ok(PresentationContextDefinitionList::new(
            self.context_id
                .ok_or(PresentationContextDefinitionListBuilderError::NoContextId)?,
            self.abstract_syntax_item
                .ok_or(PresentationContextDefinitionListBuilderError::NoAbstractSyntax)?,
            self.transfer_syntax_items,
        ))
    }
}

#[derive(Debug)]
pub enum AssociateRequestResponse {
    Accepted(AcceptedAssociateRequestResponse),
    Rejected(RejectedAssociateRequestResponse),
}

#[derive(Debug)]
pub struct AcceptedAssociateRequestResponse {
    pub context_name: String,
    pub called_ae: String,
    pub calling_ae: String,
    pub user_information: Vec<UserInformation>,
    pub presentation_context_result: Vec<PresentationContextDefinitionResultList>,
}

impl AcceptedAssociateRequestResponse {
    pub fn new(
        context_name: String,
        called_ae: String,
        calling_ae: String,
        user_information: Vec<UserInformation>,
        presentation_context_result: Vec<PresentationContextDefinitionResultList>,
    ) -> Self {
        Self {
            context_name,
            called_ae,
            calling_ae,
            user_information,
            presentation_context_result,
        }
    }

    pub fn presentation_context_result(&self) -> &Vec<PresentationContextDefinitionResultList> {
        &self.presentation_context_result
    }

    pub fn user_information(&self) -> &Vec<UserInformation> {
        &self.user_information
    }
}

#[derive(Debug)]
pub struct RejectedAssociateRequestResponse {
    pub diagnostic: Option<ServiceUserReason>,
    pub result: RejectedAssociationResult,
}

impl RejectedAssociateRequestResponse {
    pub fn new(diagnostic: Option<ServiceUserReason>, result: RejectedAssociationResult) -> Self {
        Self { diagnostic, result }
    }
}

#[cfg(test)]
mod tests {
    use std::string::String;

    use crate::associate::presentation_context::PresentationContextResult;
    use crate::service::{
        PresentationContextDefinitionList, PresentationContextDefinitionListBuilder,
        PresentationContextDefinitionListBuilderError, PresentationContextDefinitionResultList,
    };

    #[test]
    fn test_presentation_context_definition_result_list() {
        let definition_list_id = 1;
        let definition_list_abstract = String::from("1.2.840.10008.1.1");
        let definition_list_transfer = vec![String::from("1.2.840.10008.1.2")];

        let definition_list = PresentationContextDefinitionList::new(
            definition_list_id.clone(),
            definition_list_abstract.clone(),
            definition_list_transfer.clone(),
        );

        let result = PresentationContextResult::Acceptance;
        let result_list = PresentationContextDefinitionResultList::from_definition_list(
            definition_list,
            result.clone(),
        );

        assert_eq!(result_list.context_id, definition_list_id);
        assert_eq!(result_list.abstract_syntax, definition_list_abstract);
        assert_eq!(result_list.transfer_syntax, definition_list_transfer);
        assert_eq!(result_list.result, result);
    }

    #[test]
    fn test_definition_list_builder_ok() {
        let result = PresentationContextDefinitionListBuilder::new()
            .context_id(1)
            .abstract_syntax("1.2.840".to_string())
            .add_transfer_syntax("ts1".to_string())
            .build()
            .unwrap();

        assert_eq!(result.context_id, 1);
        assert_eq!(result.abstract_syntax, "1.2.840");
        assert_eq!(result.transfer_syntax, vec!["ts1"]);
    }

    #[test]
    fn test_definition_list_builder_multiple_transfer_ok() {
        let result = PresentationContextDefinitionListBuilder::new()
            .context_id(1)
            .abstract_syntax("1.2.840".to_string())
            .add_transfer_syntax("ts1".to_string())
            .add_transfer_syntax("ts2".to_string())
            .build()
            .unwrap();

        assert_eq!(result.context_id, 1);
        assert_eq!(result.abstract_syntax, "1.2.840");
        assert_eq!(result.transfer_syntax, vec!["ts1", "ts2"]);
    }

    #[test]
    fn test_definition_list_builder_transfer_syntax_overrides_previous() {
        let result = PresentationContextDefinitionListBuilder::new()
            .context_id(3)
            .abstract_syntax("abc".to_string())
            .add_transfer_syntax("old".to_string())
            .transfer_syntax(vec!["new1".to_string(), "new2".to_string()])
            .build()
            .unwrap();

        assert_eq!(result.transfer_syntax, vec!["new1", "new2"]);
    }

    #[test]
    fn test_definition_list_builder_missing_context() {
        assert!(matches!(
            PresentationContextDefinitionListBuilder::new()
                .abstract_syntax("1.2.840".to_string())
                .add_transfer_syntax("ts1".to_string())
                .build(),
            Err(PresentationContextDefinitionListBuilderError::NoContextId)
        ));
    }

    #[test]
    fn test_definition_list_builder_missing_abstract_syntax() {
        assert!(matches!(
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .add_transfer_syntax("ts1".to_string())
                .build(),
            Err(PresentationContextDefinitionListBuilderError::NoAbstractSyntax)
        ));
    }

    #[test]
    fn test_definition_list_builder_missing_transfer_syntax() {
        assert!(matches!(
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840".to_string())
                .build(),
            Err(PresentationContextDefinitionListBuilderError::NoTransferSyntax)
        ));
    }
}
