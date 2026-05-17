use strum_macros::{IntoStaticStr, Display};

use thiserror::Error;

use crate::{
    DeserializedPdu, ul::associate::{AssociateRqAcPdu, abort::AssociateAbortPdu}, ul::service::{
        AbortIndication, AcceptedAssociateRequestResponse, AssociateRequestIndication, ProviderAbortIndication, RejectedAssociateRequestResponse
    }
};

/// DICOM standard events
#[derive(Debug, PartialEq)]
pub enum Event {
    TransportConnectionIndication,
    ConnectionOpen,
    AssociateRequestPdu(AssociateRqAcPdu),
    AssociateRejectPdu,
    AssociateAcceptPdu(AssociateRqAcPdu),
    DataPdu,
    AssociateAbortPdu(AssociateAbortPdu),
    AssociateRequestPrimitive(AssociateRequestIndication),
    AssociateResponsePrimitiveReject(RejectedAssociateRequestResponse),
    AssociateResponsePrimitiveAccept(AcceptedAssociateRequestResponse),
    TransportConnectionClosedIndication,

    // Abort events
    UnrecognizedPdu,
    UnexpectedPdu,
    UnrecognizedPduParameter,
    UnexpectedPduParameter,
    InvalidPduParameter
}

/// Commands that the system should perform. Typically spawned in the case of an [Event].
#[derive(IntoStaticStr, Display, Debug)]
pub enum Command {
    AssociateIndication(AssociateRequestIndication),
    AbortIndication(AbortIndication),
    ProviderAbortIndication(ProviderAbortIndication),
    AssociateResponse(RejectedAssociateRequestResponse),
    AssociateAcceptPdu(AssociateRqAcPdu),
    AssociateRequestPdu(AssociateRqAcPdu),
    AbortPdu(AssociateAbortPdu),
    StartArtimTimer,
    OpenConnection,
}

#[derive(Debug, Error)]
pub enum IndicationError {
    #[error("Command not an indication; {0}")]
    InvalidCommand(Command),
}

#[derive(IntoStaticStr)]
pub enum Indication {
    AssociateIndication(AssociateRequestIndication),
    AbortIndication(AbortIndication),
    ProviderAbortIndication(ProviderAbortIndication)
}

impl Indication {
    pub fn from_command(cmd: Command) -> Result<Self, IndicationError> {
        match cmd {
            Command::AssociateIndication(inner) => {
                Ok(Self::AssociateIndication(inner))
            }

            Command::AbortIndication(inner) => {
                Ok(Self::AbortIndication(inner))
            }

            _ => {
                Err(IndicationError::InvalidCommand(cmd))
            }
        }
    }
}

pub fn event_from_deserialized_pdu(pdu: DeserializedPdu) -> Event {
    match pdu {
        DeserializedPdu::AssociateRequest(inner) => Event::AssociateRequestPdu(inner),
        DeserializedPdu::Abort(inner) => Event::AssociateAbortPdu(inner),
        DeserializedPdu::AssociateAccept(inner) => Event::AssociateAcceptPdu(inner),
        _ => todo!()
    }
}
