use std::io::Read;
use std::net::IpAddr;

use async_trait::async_trait;

use eradic_common::Pdu;
use eradic_common::associate::{AssociateRqAcPdu, deserialize_association_pdu, rj::ServiceUserReason};
use eradic_common::event::{Command, Event};
use eradic_common::service::{self, AssociateRequestIndication, AssociateRequestResponse};
use eradic_common::connection::UpperLayerAcceptorConnection;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait UpperLayerServiceUserAsync: Send + Sync {
    async fn handle_associate_request(
        &mut self,
        pdu: AssociateRequestIndication,
    ) -> AssociateRequestResponse;
}

pub trait UpperLayerServiceUser: Send + Sync {
    fn handle_associate_request(
        &mut self,
        pdu: AssociateRequestIndication,
    ) -> AssociateRequestResponse;
}

/// Helper function for handling DICOM state. Handles some commands and returns the rest.
///
/// Part of the DICOM Upper Layer protocol. Intended to be agnostic of networking implementation and how PDU's are read.
///
///
/// # Examples
///
/// ```
/// use eradic_adaptor::handle_incoming_pdu_async;
/// command = handle_incoming_pdu(pdu, &mut conn, service_user).unwrap();
/// ```
pub async fn handle_incoming_pdu_async<U: UpperLayerServiceUserAsync>(
    pdu: Pdu,
    conn: &mut UpperLayerAcceptorConnection,
    service_user: &mut U,
) -> Result<Option<Command>> {
    let mut command: Option<Command> = None;

    match pdu {
        Pdu::AssociationRequest(pdu) => {
            command = conn.handle_event(Event::AssociateRequestPdu(pdu))?;
        }
        _ => {
            todo!()
        }
    }

    match command {
        Some(Command::AssociationIndication(indication)) => {
            let response = service_user.handle_associate_request(indication).await;

            match response {
                AssociateRequestResponse::Accepted(inner) => {
                    // Handle the event and return the next command

                    conn.handle_event(Event::AssociateResponsePrimitiveAccept(inner))
                }
                AssociateRequestResponse::Rejected(inner) => {
                    conn.handle_event(Event::AssociateResponsePrimitiveReject(inner))
                }
            }
        }
        None => Ok(None),
        _ => Ok(None),
    }
}
