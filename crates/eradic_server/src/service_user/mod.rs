use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

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

use eradic_adaptor::{ServiceUserError};

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

pub struct LocalUpperLayerServiceUser {
    application_entities: ApplicationEntityRegistry,
}

impl LocalUpperLayerServiceUser {
    pub fn new() -> Self {
        let mut registry = ApplicationEntityRegistry::new();
        registry.insert("rad".to_string(), Arc::new(Mutex::new(Pacs {})));
        Self { application_entities: registry }
    }

    pub async fn handle_associate_request(&self, indication: AssociateRequestIndication) -> Event {
        match self.application_entities.get(&indication.called_ae) {
            Some(pacs) => {
                let event = {
                    let mut guard = pacs.lock().unwrap();
                    guard.handle_associate_request(indication)
                };
                event
            },
            _ => todo!(),
        }
    }
}
