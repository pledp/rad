use crate::{
    ul::associate::{
        ApplicationContextItem, AssociateItemType, AssociateRqAcPdu, AssociateRqAcPduError, PduDeserializationError, UserInfoItem, UserInformationSubItem, presentation_context::{
            PresentationContextItemBuilder, SyntaxItemBuilder
        }
    }, ul::pdu::PduType, ul::service::AcceptedAssociateRequestResponse
};
use crate::ul::service::{AssociateRequestResponse, AssociateRequestIndication};

impl TryFrom<AssociateRequestIndication> for AssociateRqAcPdu {
    type Error = PduDeserializationError;

    fn try_from(indication: AssociateRequestIndication) -> Result<Self, Self::Error> {
        const NO_VARIABLE_FIELDS_LENGTH: u32 = 68;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let application_context_item = ApplicationContextItem::new(&indication.context_name);
        length += application_context_item.item_length();

        let mut presentation_context_items = Vec::new();

        for context in indication.presentation_context() {
            let mut builder = PresentationContextItemBuilder::new()
                .item_type(AssociateItemType::PresentationContextRq)
                .context_id(context.context_id)
                .abstract_syntax_item(
                    SyntaxItemBuilder::new()
                        .item_type(AssociateItemType::AbstractSyntax)
                        .syntax(context.abstract_syntax.clone())
                        .build()?,
                );

            for transfer in &context.transfer_syntax {
                builder = builder.add_transfer_syntax(
                    SyntaxItemBuilder::new()
                        .item_type(AssociateItemType::TransferSyntax)
                        .syntax(transfer)
                        .build()?,
                );
            }

            presentation_context_items.push(builder.build()?);
        }

        length += presentation_context_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        let user_info_sub_items = indication
            .user_information()
            .iter()
            .map(|ui| UserInformationSubItem::new(*ui))
            .collect();

        let user_info_item = UserInfoItem::new(user_info_sub_items);
        length += user_info_item.item_length();

        Ok(Self {
            pdu_type: PduType::AssociateRequest,
            length,
            protocol_version: 1,
            called_ae: indication.called_ae.clone(),
            calling_ae: indication.calling_ae.clone(),
            application_context_item,
            presentation_context_items,
            user_info_item,
        })
    }
}

impl TryFrom<AcceptedAssociateRequestResponse> for AssociateRqAcPdu {
    type Error = AssociateRqAcPduError;

    fn try_from(response: AcceptedAssociateRequestResponse) -> Result<Self, Self::Error> {
        const NO_VARIABLE_FIELDS_LENGTH: u32 = 68;
        let mut length = NO_VARIABLE_FIELDS_LENGTH;

        let application_context_item = ApplicationContextItem::new(&response.context_name);
        length += application_context_item.item_length();

        let mut presentation_context_items = Vec::new();

        for context in response.presentation_context_result() {
            presentation_context_items.push(
                PresentationContextItemBuilder::new()
                    .item_type(AssociateItemType::PresentationContextAc)
                    .context_id(context.context_id)
                    .result(context.result)
                    .add_transfer_syntax(
                        SyntaxItemBuilder::new()
                            .item_type(AssociateItemType::TransferSyntax)
                            .syntax(context.transfer_syntax.clone())
                            .build()?,
                    )
                    .build()?,
            );
        }

        length += presentation_context_items
            .iter()
            .map(|item| item.item_length())
            .sum::<u32>();

        let user_info_sub_items = response
            .user_information()
            .iter()
            .map(|ui| UserInformationSubItem::new(*ui))
            .collect();

        let user_info_item = UserInfoItem::new(user_info_sub_items);
        length += user_info_item.item_length();

        Ok(Self {
            pdu_type: PduType::AssociateAccept,
            length,
            protocol_version: 1,
            called_ae: response.called_ae.clone(),
            calling_ae: response.calling_ae.clone(),
            application_context_item,
            presentation_context_items,
            user_info_item,
        })
    }
}
