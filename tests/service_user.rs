use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use eradic::ul::service::PresentationContextDefinitionResult;
use eradic::{
    ul::associate::{
        presentation_context::PresentationContextResult,
    },
    ul::event::Event,
    ul::service::{
        AssociateRequestResponsePrimitive, AssociateRequestIndicationPrimitive,
    },
};

pub type ApplicationEntityRegistry = HashMap<String, Arc<Mutex<Pacs>>>;

struct Pacs {}

impl Pacs {
    pub fn handle_associate_request(&mut self, indication: AssociateRequestIndicationPrimitive) -> AssociateRequestResponsePrimitive {
        let presentation_context_result = indication
            .presentation_context()
            .iter()
            .map(|item| PresentationContextDefinitionResult {
                context_id: item.context_id,
                transfer_syntax: item.transfer_syntax[0].clone(),
                result: PresentationContextResult::Acceptance,
            })
            .collect();

        AssociateRequestResponsePrimitive {
            context_name: indication.context_name,
            called_ae: indication.called_ae,
            calling_ae: indication.calling_ae,
            user_information: indication.user_information,
            presentation_context_result,
            diagnostic: None,
            result: None,
        }
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

    pub async fn handle_associate_request(&self, indication: AssociateRequestIndicationPrimitive) -> AssociateRequestResponsePrimitive {
        match self.application_entities.get(&indication.called_ae) {
            Some(pacs) => {
                let prim = {
                    let mut guard = pacs.lock().unwrap();
                    guard.handle_associate_request(indication)
                };

                prim
            },
            _ => todo!(),
        }
    }
}
