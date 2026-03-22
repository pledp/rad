use std::net::IpAddr;
use std::io::Read;

use async_trait::async_trait;

use rad_common::service::AssociateRequestIndication;
use rad_common::associate::{AssociateRqAcPdu, deserialize_association_pdu, AssociationResult};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[async_trait]
pub trait UpperLayerServiceUser: Send + Sync {
    async fn handle_associate_request(&mut self, pdu: AssociateRequestIndication) -> AssociationResult;
}

pub fn issue_indication<R: Read>(reader: &mut R, calling: IpAddr, called: IpAddr) -> Result<AssociateRequestIndication> {
    let pdu = deserialize_association_pdu(reader)?;
    issue_indication_from_pdu(pdu, calling, called)
}

pub fn issue_indication_from_pdu(pdu: AssociateRqAcPdu, called: IpAddr, calling: IpAddr) -> Result<AssociateRequestIndication> {
    Ok(AssociateRequestIndication::from_rq_pdu(
        pdu,
        called,
        calling
    ))
}
