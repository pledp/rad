use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use tokio::sync::{Mutex};

use eradic_common::service::PresentationContextDefinitionResultList;
use eradic_common::{
    associate::{
        presentation_context::PresentationContextResult,
    },
    event::Event,
    service::{
        AcceptedAssociateRequestResponse, AssociateRequestIndication,
    },
};

use eradic_adaptor::{ServiceUserError, UpperLayerServiceUser, UpperLayerServiceUserAsync, UpperLayerServiceUserConnection, UpperLayerServiceUserConnectionAsync};

pub type ApplicationEntityRegistry = HashMap<String, Arc<Mutex<Pacs>>>;

struct Pacs {}

impl Pacs {
    pub fn handle_associate_request(&mut self, indication: AssociateRequestIndication) -> Event {
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


pub struct LocalUpperLayerServiceUserConnection {
    pacs: Arc<Mutex<Pacs>>
}

#[async_trait]
impl UpperLayerServiceUserConnectionAsync for LocalUpperLayerServiceUserConnection {
    async fn handle_associate_request(&mut self, indication: AssociateRequestIndication) -> Event {
        let mut guard = self.pacs.lock().await;
        (*guard).handle_associate_request(indication)
    }
}

pub struct LocalUpperLayerServiceUser {
    application_entities: ApplicationEntityRegistry,
}

impl LocalUpperLayerServiceUser {
    pub fn new() -> Self {
        let mut registry = ApplicationEntityRegistry::new();
        registry.insert("rad".to_string(), Arc::new(Mutex::new(Pacs {})));
        Self { application_entities: registry }
    }
}

impl UpperLayerServiceUserAsync for LocalUpperLayerServiceUser {
    fn create_scu_connection(
        &self,
        ae: &str,
    ) -> Result<Box<dyn UpperLayerServiceUserConnectionAsync>, ServiceUserError> {
        match self.application_entities.get(ae) {
            Some(pacs) => Ok(Box::new(LocalUpperLayerServiceUserConnection {
                pacs: Arc::clone(pacs),
            })),
            None => Err(ServiceUserError::ServiceUserNotFound),
        }
    }
}
