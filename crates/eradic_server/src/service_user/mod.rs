use std::collections::HashMap;

use thiserror::Error;

use eradic_common::service::PresentationContextDefinitionResultList;
use eradic_common::{
    associate::{
        RejectedAssociateResult, presentation_context::PresentationContextResult,
        rj::ServiceUserReason,
    },
    event::Event,
    service::{
        AcceptedAssociateRequestResponse, AssociateRequestIndication,
        RejectedAssociateRequestResponse,
    },
};

use eradic_adaptor::UpperLayerServiceUserConnection;

pub type ApplicationEntityRegistry = Vec<String>;

struct Pacs {}

impl UpperLayerServiceUserConnection for Pacs {
    fn handle_associate_request(&mut self, indication: AssociateRequestIndication) -> Event {
        let presentation_context_result = indication
            .presentation_context
            .into_iter()
            .map(|ctx| {
                PresentationContextDefinitionResultList::from_definition_list(
                    ctx,
                    PresentationContextResult::Acceptance,
                )
            })
            .collect();

        Event::AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse {
            context_name: indication.context_name,
            called_ae: indication.called_ae,
            calling_ae: indication.calling_ae,
            user_information: indication.user_information,
            presentation_context_result,
        })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceUserError {
    #[error("Service User / Application Entity was not ")]
    ServiceUserNotFound,
}

pub struct UpperLayerServiceUser {
    //application_entities: ApplicationEntityRegistry,
}

impl UpperLayerServiceUser {
    pub fn new() -> Self {
        Self {}
    }

    pub fn create_scu_connection(
        ae: &str,
    ) -> Result<Box<dyn UpperLayerServiceUserConnection>, ServiceUserError> {
        match ae {
            "rad" => Ok(Box::new(Pacs {})),
            _ => Err(ServiceUserError::ServiceUserNotFound),
        }
    }
}
