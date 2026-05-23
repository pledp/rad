pub mod service;
pub mod connection;
pub mod event;
pub mod associate;
pub mod pdu;

pub struct UpperLayerConfiguration {
    /// DICOM standard ARTIM (Association Request/Reject/Relase Timer) is used as a timeout for hung
    /// TCP connections.
    ///
    /// This field is used to configure the timeout in milliseconds.
    artim_timeout: u32
}
