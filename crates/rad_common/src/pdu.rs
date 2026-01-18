#[repr(u8)]
enum Type {
    AssociateRq = 0x01,
    AssociateAc = 0x02,
    AssociateRj = 0x03,
    Data = 0x04,
    ReleaseRq = 0x05,
    ReleaseRp = 0x06,
    Abort = 0x07
}

/// Events related to A-ASSOCIATE. Events lead to actions defined by the DICOM standard.
/// ISO/TR 2382:2015 defines primitives. Primitives are abstract interactions between a service user and a service provider. 
/// In DICOM, primitives are interactions between the DICOM server (service provider) and the client (service user).
/// 
/// See [DICOM standard part 8 subsection 9](https://dicom.nema.org/medical/dicom/current/output/html/part08.html#sect_9).
enum AssociationEvent {
    PrimitiveRequestAssociation,
    PrimitiveResponseAccept,
    PrimitiveResponseReject,
    PrimitiveConfirmTransport,
    PrimitiveIndicationTransport,
    AssociationRequest,
    AssociationAccept,
    AssociationReject
}

/* 
enum State {
    Idle,
    _.
    _.
    AwaitTransportConnection,
    AwaitAcRjPdu
}

*/
