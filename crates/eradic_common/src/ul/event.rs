use std::net::IpAddr;

use strum_macros::{EnumDiscriminants, IntoStaticStr, Display};

use thiserror::Error;

use crate::{
    DeserializedPdu, ul::{associate::{rq_ac::AssociateRqAcPdu, abort::AssociateAbortPdu, rj::AssociateRjPdu}, service::{
        AbortIndicationPrimitive, AcceptedAssociateConfirmationPrimitive, RejectedAssociateConfirmationPrimitive, AssociateRequestIndicationPrimitive, AssociateResponsePrimitive, ProviderAbortIndicationPrimitive
    }}
};

/// DICOM standard events
#[derive(Debug, Clone, PartialEq, IntoStaticStr, EnumDiscriminants)]
#[strum_discriminants(name(EventKind), derive(serde::Deserialize, Hash))]
pub enum Event {
    TransportConnectionConfirmation {
        called_address: IpAddr,
        called_port: u16,
        calling_address: IpAddr,
        calling_port: u16,
    },

    TransportConnectionIndication {
        called_address: IpAddr,
        called_port: u16,
        calling_address: IpAddr,
        calling_port: u16,
    },

    AssociateRequestPdu(AssociateRqAcPdu),
    AssociateRejectPdu(AssociateRjPdu),
    AssociateAcceptPdu(AssociateRqAcPdu),
    DataPdu,
    AssociateAbortPdu(AssociateAbortPdu),
    AssociateRequestPrimitive(AssociateRequestIndicationPrimitive),
    AcceptedAssociateResponsePrimitive(AssociateResponsePrimitive),
    RejectedAssociateResponsePrimitive(AssociateResponsePrimitive),
    TransportConnectionClosedIndication,

    // Abort events
    UnrecognizedPdu,
    UnexpectedPdu,
    UnrecognizedPduParameter,
    UnexpectedPduParameter,
    InvalidPduParameter,
    AbortRequestPrimitive,

    ArtimTimerExpired,
}

/// Commands that the system should perform. Typically spawned in the case of an [Event].
#[derive(IntoStaticStr, Display, Debug, EnumDiscriminants)]
#[strum_discriminants(name(CommandKind), derive(serde::Deserialize))]
pub enum Command {
    AssociateIndicationPrimitive(AssociateRequestIndicationPrimitive),
    AbortIndicationPrimitive(AbortIndicationPrimitive),
    ProviderAbortIndicationPrimitive(ProviderAbortIndicationPrimitive),
    AssociateAcceptPdu(AssociateRqAcPdu),
    AssociateRejectPdu(AssociateRjPdu),
    AssociateRequestPdu(AssociateRqAcPdu),

    AcceptedAssociateConfirmationPrimitive(AcceptedAssociateConfirmationPrimitive),
    RejectedAssociateConfirmationPrimitive(RejectedAssociateConfirmationPrimitive),
    TransportConnectionRequest(String),

    // Association Abort Related Actions/Commands
    CloseConnection,
    /// Generic command to send AbortPdu
    AbortPdu(AssociateAbortPdu),
    /// Generic command to stop ARTIM timer
    StopArtimTimer,
    /// Generic command to start ARTIM timer
    StartArtimTimer,
}

#[derive(Debug, Error)]
pub enum IndicationError {
    #[error("Command not an indication; {0}")]
    InvalidCommand(Command),
}

pub enum ServiceUserToServiceProvider {
    AbortRequest,
    AssociateRequestPrimitive(AssociateRequestIndicationPrimitive),
    AssociateResponsePrimitive(AssociateResponsePrimitive),
}

pub type Request = ServiceUserToServiceProvider;
pub type Response = ServiceUserToServiceProvider;

#[derive(IntoStaticStr)]
pub enum ServiceProviderToServiceUser {
    AssociateIndicationPrimitive(AssociateRequestIndicationPrimitive),
    AcceptedAssociateConfirmationPrimitive(AcceptedAssociateConfirmationPrimitive),
    RejectedAssociateConfirmationPrimitive(RejectedAssociateConfirmationPrimitive),
    AbortIndicationPrimitive(AbortIndicationPrimitive),
    ProviderAbortIndicationPrimitive(ProviderAbortIndicationPrimitive)
}

pub type Indication = ServiceProviderToServiceUser;
pub type Confirmation = ServiceProviderToServiceUser;

impl ServiceProviderToServiceUser {
    pub fn from_command(cmd: Command) -> Result<Self, IndicationError> {
        match cmd {
            Command::AssociateIndicationPrimitive(inner) => {
                Ok(Self::AssociateIndicationPrimitive(inner))
            }

            Command::AbortIndicationPrimitive(inner) => {
                Ok(Self::AbortIndicationPrimitive(inner))
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
        DeserializedPdu::AssociateReject(inner) => Event::AssociateRejectPdu(inner),
        _ => todo!()
    }
}
