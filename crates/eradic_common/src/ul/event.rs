use std::net::IpAddr;

use strum_macros::{EnumDiscriminants, IntoStaticStr, Display};

use thiserror::Error;

use crate::{
    DeserializedPdu, ul::{associate::{AssociateRqAcPdu, abort::AssociateAbortPdu, rj::AssociateRjPdu}, service::{
        AbortIndication, AssociateConfirmation, AssociateRequestIndication, AssociateRequestResponse, ProviderAbortIndication
    }}
};

/// DICOM standard events
#[derive(Debug, PartialEq, IntoStaticStr, EnumDiscriminants)]
#[strum_discriminants(name(EventKind), derive(serde::Deserialize, Hash))]
pub enum Event {
    AssociateRequest(AssociateRequestIndication),
    ConnectionOpen {
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
    AssociateRequestPrimitive(AssociateRequestIndication),
    AssociateResponsePrimitive(AssociateRequestResponse),
    TransportConnectionClosedIndication,

    // Abort events
    UnrecognizedPdu,
    UnexpectedPdu,
    UnrecognizedPduParameter,
    UnexpectedPduParameter,
    InvalidPduParameter,
    AbortRequest,

    ArtimTimerExpired,
}

/// Commands that the system should perform. Typically spawned in the case of an [Event].
#[derive(IntoStaticStr, Display, Debug, EnumDiscriminants)]
#[strum_discriminants(name(CommandKind), derive(serde::Deserialize))]
pub enum Command {
    AssociateIndication(AssociateRequestIndication),
    AbortIndication(AbortIndication),
    ProviderAbortIndication(ProviderAbortIndication),
    AssociateAcceptPdu(AssociateRqAcPdu),
    AssociateRequestPdu(AssociateRqAcPdu),

    AssociateConfirmation(AssociateConfirmation),
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
    Event(Event)
}

pub type Request = ServiceUserToServiceProvider;
pub type Response = ServiceUserToServiceProvider;

#[derive(IntoStaticStr)]
pub enum ServiceProviderToServiceUser {
    AssociateIndication(AssociateRequestIndication),
    AssociateConfirmation(AssociateConfirmation),
    AbortIndication(AbortIndication),
    ProviderAbortIndication(ProviderAbortIndication)
}

pub type Indication = ServiceProviderToServiceUser;
pub type Confirmation = ServiceProviderToServiceUser;

impl ServiceProviderToServiceUser {
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
        DeserializedPdu::AssociateReject(inner) => Event::AssociateRejectPdu(inner),
        _ => todo!()
    }
}
