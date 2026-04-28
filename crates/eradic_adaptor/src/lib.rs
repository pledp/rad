use async_trait::async_trait;
use thiserror::Error;

use eradic_common::event::Event;
use eradic_common::service::AssociateRequestIndication;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceUserError {
    #[error("Service User / Application Entity was not ")]
    ServiceUserNotFound,
}

pub trait UpperLayerServiceUser {
    fn handle_associate_request(&self, pdu: AssociateRequestIndication) -> Event;
}

#[async_trait]
pub trait UpperLayerServiceUserAsync {
    async fn handle_associate_request(&self, pdu: AssociateRequestIndication) -> Event;
}
