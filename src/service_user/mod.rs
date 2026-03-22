use async_trait::async_trait;

use std::collections::HashMap;

use rad_common::{
    associate::rj::{AcseReason, PresentationReason, RejectReason, RejectResult, RejectSource, ServiceUserReason},
    service::{
        AssociateRequestIndication
    },
    associate::AssociationResult,
};

use eradic_adaptor::{UpperLayerServiceUser};

pub type ApplicationEntityRegistry = HashMap<String, Box<dyn ApplicationEntity>>;

trait ApplicationEntity: Send + Sync {
    fn handle_associate_request(&self, indication: AssociateRequestIndication) -> AssociationResult;
}

struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(&self, indication: AssociateRequestIndication) -> AssociationResult {
        if indication.context_name != "1.2.840.10008.3.1.1.1" {

        }

        for presentation_context_item in indication.presentation_context {

        }

        AssociationResult::Accepted
    }
}

pub struct ServiceUser {
    application_entities: ApplicationEntityRegistry
}

impl ServiceUser {
    pub fn new() -> Self {
        let mut application_entities: ApplicationEntityRegistry = HashMap::new();
        application_entities.insert("rad".into(), Box::new(Pacs {}));

        Self {
            application_entities
        }
    }
}

#[async_trait]
impl UpperLayerServiceUser for ServiceUser {
    async fn handle_associate_request(&mut self, indication: AssociateRequestIndication) -> AssociationResult {
        let result = self.application_entities
            .get(&indication.called_ae)
            .map(|ae| ae.handle_associate_request(indication));

        match result {
            Some(result) => {
                result
            }
            None => {
                AssociationResult::Rejected {
                    result: RejectResult::Permanent,
                    source: RejectSource::ServiceUser,
                    reason: RejectReason::ServiceUser(ServiceUserReason::CalledAeNotRecognized)
                }
            }
        }
    }
}
