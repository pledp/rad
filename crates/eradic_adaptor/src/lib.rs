use async_trait::async_trait;
use thiserror::Error;

use eradic_common::event::Event;
use eradic_common::service::AssociateRequestIndication;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceUserError {
    #[error("Service User / Application Entity was not ")]
    ServiceUserNotFound,
}

#[async_trait]
pub trait UpperLayerServiceUserConnectionAsync: Send + Sync {
    async fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> Event;
}

pub trait UpperLayerServiceUserConnection: Send + Sync {
    fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> Event;
}

pub trait UpperLayerServiceUser {
    fn create_scu_connection(
        &self,
        ae: &str,
    ) -> Result<Box<dyn UpperLayerServiceUserConnection>, ServiceUserError>;
}

pub trait UpperLayerServiceUserAsync {
    fn create_scu_connection(
        &self,
        ae: &str,
    ) -> Result<Box<dyn UpperLayerServiceUserConnectionAsync>, ServiceUserError>;
}
