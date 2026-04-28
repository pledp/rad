use thiserror::Error;

use eradic_common::event::Event;
use eradic_common::service::AssociateRequestIndication;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ServiceUserError {
    #[error("Service User / Application Entity was not ")]
    ServiceUserNotFound,
}
