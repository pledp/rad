/// The PACS DICOM application entity.
use crate::{ApplicationEntity, AssociateResult};

pub struct Pacs {}

impl ApplicationEntity for Pacs {
    fn handle_associate_request(&self) -> AssociateResult {
        AssociateResult::Accepted
    }
}
