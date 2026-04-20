use std::io::Read;
use std::net::IpAddr;

use async_trait::async_trait;

use eradic_common::DeserializedPdu;
use eradic_common::associate::{AssociateRqAcPdu, deserialize_association_pdu, rj::ServiceUserReason};
use eradic_common::event::{Command, Event};
use eradic_common::service::{self, AssociateRequestIndication, AssociateRequestResponse};
use eradic_common::connection::{UpperLayerAcceptorConnection, handle_server_event};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait UpperLayerServiceUserAsync: Send + Sync {
    async fn handle_associate_request(
        &mut self,
        pdu: AssociateRequestIndication,
    ) -> Event;
}

pub trait UpperLayerServiceUser: Send + Sync {
    fn handle_associate_request(
        &mut self,
        pdu: AssociateRequestIndication,
    ) -> Event;
}
