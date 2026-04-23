use async_trait::async_trait;

use eradic_common::event::Event;
use eradic_common::service::AssociateRequestIndication;

#[async_trait]
pub trait UpperLayerServiceUserConnectionAsync: Send + Sync {
    async fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> Event;
}

pub trait UpperLayerServiceUserConnection: Send + Sync {
    fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> Event;
}
