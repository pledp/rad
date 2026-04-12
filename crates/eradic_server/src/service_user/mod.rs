use async_trait::async_trait;

use std::collections::HashMap;

use eradic_common::{
    associate::{
        RejectedAssociationResult, presentation_context::PresentationContextResult, rj::{AcseReason, PresentationReason, RejectReason, RejectSource, ServiceUserReason}
    },
    service::{
        AcceptedAssociateRequestResponse, AssociateRequestIndication, AssociateRequestResponse,
        RejectedAssociateRequestResponse, presentation_context_definition_list_with_result
    },
};

use eradic_adaptor::UpperLayerServiceUserAsync;

pub type ApplicationEntityRegistry = HashMap<String, Box<dyn ApplicationEntity>>;

trait ApplicationEntity: Send + Sync {
    fn handle_associate_request(
        &self,
        indication: AssociateRequestIndication,
    ) -> AssociateRequestResponse;
}

struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(
        &self,
        indication: AssociateRequestIndication,
    ) -> AssociateRequestResponse {
        let presentation_context_result = vec![
            presentation_context_definition_list_with_result(
                &indication.presentation_context[0],
                PresentationContextResult::Acceptance
            )
        ];

        AssociateRequestResponse::Accepted(AcceptedAssociateRequestResponse {
            context_name: indication.context_name,
            called_ae: indication.called_ae,
            calling_ae: indication.calling_ae,
            user_information: indication.user_information,
            presentation_context_result,
        })
    }
}

pub struct ServiceUser {
    application_entities: ApplicationEntityRegistry,
}

impl ServiceUser {
    pub fn new() -> Self {
        let mut application_entities: ApplicationEntityRegistry = HashMap::new();
        application_entities.insert("rad".into(), Box::new(Pacs {}));

        Self {
            application_entities,
        }
    }
}

#[async_trait]
impl UpperLayerServiceUserAsync for ServiceUser {
    async fn handle_associate_request(
        &mut self,
        indication: AssociateRequestIndication,
    ) -> AssociateRequestResponse {
        let result = self
            .application_entities
            .get(&indication.called_ae)
            .map(|ae| ae.handle_associate_request(indication));

        match result {
            Some(result) => result,
            None => AssociateRequestResponse::Rejected(RejectedAssociateRequestResponse::new(
                Some(ServiceUserReason::CalledAeNotRecognized),
                RejectedAssociationResult::RejectedPermanent,
            )),
        }
    }
}
