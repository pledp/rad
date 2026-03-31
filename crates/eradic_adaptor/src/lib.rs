pub mod association;

use std::io::Read;
use std::net::IpAddr;

use async_trait::async_trait;

use rad_common::associate::{AssociateRqAcPdu, deserialize_association_pdu, rj::ServiceUserReason};
use rad_common::service::{self, AssociateRequestIndication, AssociateRequestResponse};

use crate::association::{UpperLayerConnection, WaitingForRequestPdu, WaitingForResponsePrimitive};

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
    fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> AssociateRequestResponse;
}

pub fn issue_indication<R: Read>(
    reader: &mut R,
    calling: IpAddr,
    called: IpAddr,
) -> Result<AssociateRequestIndication> {
    let pdu = deserialize_association_pdu(reader)?;
    issue_indication_from_pdu(pdu, calling, called)
}

pub fn issue_indication_from_pdu(
    pdu: AssociateRqAcPdu,
    called: IpAddr,
    calling: IpAddr,
) -> Result<AssociateRequestIndication> {
    Ok(AssociateRequestIndication::from_rq_pdu(
        pdu, called, calling,
    ))
}

/// Dispatch incoming PDU depending on connection state. Return the new state.
///
/// Part of the DICOM Upper Layer protocol. Intended to be agnostic of networking implementation and how PDU's are read.
///
/// For usage with [`UpperLayerServiceUserAsync`], check out [`handle_incoming_pdu_async`] instead.
///
/// # Examples
///
/// ```
/// use eradic_adaptor::handle_pdu_with_state;
/// conn = handle_incoming_pdu(&mut reader, conn, called, calling).unwrap();
/// ```
pub fn handle_incoming_pdu<R: Read, U: UpperLayerServiceUser>(
    reader: &mut R,
    mut conn: UpperLayerConnection,
    service_user: &mut U,
    called: IpAddr,
    calling: IpAddr,
) -> Result<UpperLayerConnection> {
    match conn {
        UpperLayerConnection::WaitingForRequestPdu(state) => {
            let (waiting_for_response_primitive, indication) =
                read_and_handle_association_pdu(reader, state, called, calling)?;

            service_user.handle_associate_request(indication);
        }
        _ => return Err("Invalid state".into()),
    };

    todo!();
}

/// Async version of [`handle_incoming_pdu`].
pub async fn handle_incoming_pdu_async<R: Read, U: UpperLayerServiceUserAsync>(
    reader: &mut R,
    mut conn: UpperLayerConnection,
    service_user: &mut U,
    called: IpAddr,
    calling: IpAddr,
) -> Result<(UpperLayerConnection, PduCommand)> {
    match conn {
        UpperLayerConnection::WaitingForRequestPdu(state) => {
            let (waiting_for_response_primitive, indication) =
                read_and_handle_association_pdu(reader, state, called, calling)?;

            let response = service_user.handle_associate_request(indication).await;

            let maybe_data_transfer = waiting_for_response_primitive.handle_response_primitive(&response);

            match maybe_data_transfer {
                Some(data_transfer) => {
                    conn = UpperLayerConnection::DataTransfer(data_transfer);
                    Ok((conn, PduCommand::AssociationResponse(response)))
                },
                None => {
                    conn = UpperLayerConnection::WaitForTcpClose;
                    Ok((conn, PduCommand::AssociationResponse(response)))
                }
            }
        }
        _ => return Err("Invalid state".into()),
    }
}

pub enum PduCommand {
    AssociationResponse(AssociateRequestResponse)
}

fn read_and_handle_association_pdu<R: Read>(
    reader: &mut R,
    state: WaitingForRequestPdu,
    called: IpAddr,
    calling: IpAddr,
) -> Result<(WaitingForResponsePrimitive, AssociateRequestIndication)> {
    let pdu = deserialize_association_pdu(reader)?;
    state.association_request(pdu, called, calling)
}
