use async_trait::async_trait;

use std::collections::HashMap;

use rad_common::{
    associate::{
        AssociateRqAcPdu,
        rj::{RejectReason, AcseReason, PresentationReason, RejectSource, RejectResult, ServiceUserReason}
    },
};

use eradic_adaptor::{AssociationResult, UpperLayerServiceUser};

pub type ApplicationEntityRegistry = HashMap<String, Box<dyn ApplicationEntity>>;

trait ApplicationEntity: Send + Sync {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult;
}

struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(&self, pdu: AssociateRqAcPdu) -> AssociationResult {
        if pdu.application_context_item.context_name != "1.2.840.10008.3.1.1.1" {

        }

        for presentation_context_item in pdu.presentation_context_items {

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
    async fn handle_associate_request(&mut self, pdu: AssociateRqAcPdu) -> AssociationResult {
        let result = self.application_entities
            .get(&pdu.called_ae)
            .map(|ae| ae.handle_associate_request(pdu));

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
