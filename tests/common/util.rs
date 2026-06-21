use eradic::ul::{
    associate::{
        AssociationResult,
        presentation_context::PresentationContextResult,
        rj::ServiceUserReason,
        user_information::{MaximumLength, UserInformation},
    },
    service::{
        AssociateIndication, AssociateRequest, AssociateResponsePrimitive,
        PresentationContextDefinitionListBuilder, PresentationContextDefinitionResult,
    },
};
use tokio::net::TcpStream;

pub fn default_associate_request(stream: &TcpStream) -> AssociateRequest {
    AssociateRequest::new(
        "1.2.840.10008.3.1.1.1".into(),
        "rad".into(),
        "test1".into(),
        vec![UserInformation::MaximumLength(MaximumLength {
            maximum_length: 300,
        })],
        stream.local_addr().unwrap().ip(),
        stream.local_addr().unwrap().port(),
        stream.peer_addr().unwrap().ip(),
        stream.peer_addr().unwrap().port(),
        vec![
            PresentationContextDefinitionListBuilder::new()
                .context_id(1)
                .abstract_syntax("1.2.840.10008.1.1".to_string())
                .add_transfer_syntax("1.2.840.10008.1.2".to_string())
                .build().unwrap(),
        ],
    )
}

/// Builds an [AssociateResponsePrimitive] that accepts every presentation context proposed
/// in `indication`, using whichever transfer syntax the requestor offered first for each.
pub fn accept_all_response(indication: AssociateIndication) -> AssociateResponsePrimitive {
    let presentation_context_result = indication
        .presentation_context()
        .iter()
        .map(|item| PresentationContextDefinitionResult {
            context_id: item.context_id,
            transfer_syntax: item.transfer_syntax[0].clone(),
            result: PresentationContextResult::Acceptance,
        })
        .collect();

    AssociateResponsePrimitive {
        context_name: indication.context_name,
        called_ae: indication.called_ae,
        calling_ae: indication.calling_ae,
        user_information: indication.user_information,
        presentation_context_result,
        diagnostic: ServiceUserReason::NoReason,
        result: AssociationResult::Accepted,
    }
}

/// Builds an [AssociateResponsePrimitive] that rejects the association on behalf of the
/// service user, e.g. because the calling AE title is not recognized.
pub fn reject_response(indication: AssociateIndication) -> AssociateResponsePrimitive {
    AssociateResponsePrimitive {
        context_name: indication.context_name,
        called_ae: indication.called_ae,
        calling_ae: indication.calling_ae,
        user_information: indication.user_information,
        presentation_context_result: vec![],
        diagnostic: ServiceUserReason::CallingAeNotRecognized,
        result: AssociationResult::RejectedPermanent,
    }
}
