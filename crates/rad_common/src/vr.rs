/// Value representation for a data element. The value representation describes the data type of the elements value(s).
///
/// See [DICOM standard part 5 subsection 6.2](https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_6.2).
#[derive(PartialEq, Clone)]
pub(crate) enum ValueRepresentation {
    AE,
    AS,
    AT,
    CS,
    DA,
    DS,
    DT,
    FL,
    FD,
    IS,
    LO,
    LT,
    PN,
    SH,
    SL,
    SS,
    ST,
    TM,
    UI,
    UL,
    US,
    UN,
}

impl From<&str> for ValueRepresentation {
    fn from(s: &str) -> Self {
        match s {
            "UL" => ValueRepresentation::UL,
            "UI" => ValueRepresentation::UI,
            "AE" => ValueRepresentation::AE,
            "SH" => ValueRepresentation::SH,
            _ => ValueRepresentation::UN,
        }
    }
}

/// Some data elements with specific VR have slightly different structure in memory.
///
/// See [DICOM standard part 5 subsection 7.1.2](https://dicom.nema.org/medical/dicom/current/output/html/part05.html#sect_7.1.2)
const SUBSET_VRS: [ValueRepresentation; 21] = [
    ValueRepresentation::AE,
    ValueRepresentation::AS,
    ValueRepresentation::AT,
    ValueRepresentation::CS,
    ValueRepresentation::DA,
    ValueRepresentation::DS,
    ValueRepresentation::DT,
    ValueRepresentation::FL,
    ValueRepresentation::FD,
    ValueRepresentation::IS,
    ValueRepresentation::LO,
    ValueRepresentation::LT,
    ValueRepresentation::PN,
    ValueRepresentation::SH,
    ValueRepresentation::SL,
    ValueRepresentation::SS,
    ValueRepresentation::ST,
    ValueRepresentation::TM,
    ValueRepresentation::UI,
    ValueRepresentation::UL,
    ValueRepresentation::US,
];

/// Checks if VR has 16 bit value length property.
pub(crate) fn is_16_bit_length(vr: &ValueRepresentation) -> bool {
    SUBSET_VRS.contains(vr)
}
