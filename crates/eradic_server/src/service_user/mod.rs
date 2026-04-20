use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex};

use async_trait::async_trait;

use eradic_common::event::Command;
use tokio::sync::mpsc;

use eradic_common::{
    associate::{
        RejectedAssociationResult, presentation_context::PresentationContextResult, rj::{AcseReason, PresentationReason, RejectReason, RejectSource, ServiceUserReason}
    }, event::Event, service::{
        AcceptedAssociateRequestResponse, AssociateRequestIndication, AssociateRequestResponse,
        RejectedAssociateRequestResponse, presentation_context_definition_list_with_result
    }
};

use eradic_adaptor::{UpperLayerServiceUser, UpperLayerServiceUserAsync};

pub type ApplicationEntityRegistry = HashMap<String, Box<dyn ApplicationEntity>>;

trait ApplicationEntity: Send + Sync {
    fn handle_associate_request(
        &self,
        indication: AssociateRequestIndication,
    ) -> Event;
}

struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(
        &self,
        indication: AssociateRequestIndication,
    ) -> Event {
        let presentation_context_result = vec![
            presentation_context_definition_list_with_result(
                &indication.presentation_context[0],
                PresentationContextResult::Acceptance
            )
        ];

        Event::AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse {
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

impl UpperLayerServiceUser for ServiceUser {
    fn handle_associate_request(
        &mut self,
        indication: AssociateRequestIndication,
    ) -> Event {
        let result = self
            .application_entities
            .get(&indication.called_ae)
            .map(|ae| ae.handle_associate_request(indication));

        match result {
            Some(result) => result,
            None => Event::AssociateResponsePrimitiveReject(RejectedAssociateRequestResponse::new(
                Some(ServiceUserReason::CalledAeNotRecognized),
                RejectedAssociationResult::RejectedPermanent,
            )),
        }
    }
}

pub struct UpperLayerServiceProviderConnection<U: UpperLayerServiceUser> {
    service_user: Arc<Mutex<U>>,
}

impl<U: UpperLayerServiceUser> UpperLayerServiceProviderConnection<U> {
    pub fn new(service_user: Arc<Mutex<U>>) -> Self {
        Self {
            service_user
        }
    }

    pub async fn handle_associate_request(&mut self, indication: AssociateRequestIndication, event_tx: &mut mpsc::Sender<Event>) {
        let mut guard = self.service_user.lock().await;

        let response = guard.handle_associate_request(indication);

        event_tx.send(response).await;
    }
}
