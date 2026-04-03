pub mod associate;
pub mod pdu;
pub mod service;
pub mod vr;
pub mod event;

pub use pdu::Pdu;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::vr::{ValueRepresentation, is_16_bit_length};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

const PREAMBLE_LENGTH: usize = 128;
const PREFIX_LENGTH: usize = 4;

const TAG_HALF_LENGTH: usize = 2;
const VR_LENGTH: usize = 2;
const VALUE_LENGTH: usize = 2;

const PREFIX: &str = "DICM";

#[derive(PartialEq, Eq, Hash, Clone)]
struct Tag {
    group: u16,
    element: u16,
}

impl From<(u16, u16)> for Tag {
    fn from((group, element): (u16, u16)) -> Self {
        Self { group, element }
    }
}

#[derive(Clone)]
struct DataElement {
    vr: Option<ValueRepresentation>,
    length: usize,
    raw: Option<Vec<u8>>,
}

impl DataElement {
    pub fn as_u32(&self) -> Result<u32> {
        let arr: [u8; 4] = self
            .raw
            .as_ref()
            .unwrap()
            .as_slice()
            .try_into()
            .map_err(|_| "Expected 4 bytes")?;

        Ok(u32::from_le_bytes(arr))
    }

    pub fn as_string(&self) -> Result<String> {
        let bytes = self.raw.as_ref().unwrap();
        String::from_utf8(bytes.clone()).map_err(|e| e.into())
    }
}

struct DicomFileMetadata {
    data_elements: HashMap<Tag, DataElement>,
}

impl DicomFileMetadata {
    pub fn new() -> Self {
        Self {
            data_elements: HashMap::new(),
        }
    }

    pub fn insert_data_element(&mut self, tag: Tag, element: DataElement) {
        self.data_elements.insert(tag, element);
    }

    pub fn insert_with_result(&mut self, result: DataElementResult) {
        self.insert_data_element(result.tag, result.data_element);
    }

    /// Find DICOM tag from internal hashmap.
    pub fn find_tag(&self, tag: Tag) -> Option<DataElement> {
        self.data_elements.get(&tag).cloned()
    }
}

struct DicomData {}

pub struct DicomFile {
    metadata: DicomFileMetadata,
    data: DicomData,
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<DicomFile> {
    let path = path.as_ref();
    let mut file = BufReader::new(File::open(path)?);

    let mut preamble = [0u8; PREAMBLE_LENGTH];
    file.read_exact(&mut preamble)?;

    let mut prefix = [0u8; PREFIX_LENGTH];
    file.read_exact(&mut prefix)?;

    let prefix = str::from_utf8(&prefix)?;

    if prefix != PREFIX {
        return Err("Invalid prefix".into());
    }

    let mut metadata = DicomFileMetadata::new();

    // Read first metadata tag
    metadata.insert_with_result(read_data_element(&mut file)?);

    let metadata_length = match metadata.find_tag((0x0002, 0x0000).into()) {
        Some(v) => v.as_u32()?,
        None => return Err("(0002, 0000) tag not found".into()),
    };

    let mut offset: usize = 0;

    while offset < metadata_length as usize {
        let element = read_data_element(&mut file)?;
        offset += element.length;

        let element2 = element.clone();

        metadata.insert_with_result(element);

        let data = metadata
            .find_tag((element2.tag.group, element2.tag.element).into())
            .unwrap()
            .as_string()
            .unwrap();
    }

    Ok(DicomFile {
        metadata,
        data: DicomData {},
    })
}

#[derive(Clone)]
struct DataElementResult {
    tag: Tag,
    data_element: DataElement,
    length: usize,
}

/// Read ONE data element in explicit VR Little Endian Transfer Syntax.
/// For example, File Meta Information is encoded in explicit VR little endian.
/// Data element with tag (0002, 0010) defines the Transfer Syntax for the data set.
///
/// See [DICOM standard part 10 subsection 7.1.](https://dicom.nema.org/medical/dicom/current/output/html/part10.html#sect_7.2)
fn read_data_element<T: Read>(file: &mut T) -> Result<DataElementResult> {
    let mut group_tag = [0u8; TAG_HALF_LENGTH];
    let mut element_tag = [0u8; TAG_HALF_LENGTH];
    file.read_exact(&mut group_tag)?;
    file.read_exact(&mut element_tag)?;

    // Read big endian value representation and convert to string
    let mut vr_buf = [0u8; VR_LENGTH];
    file.read_exact(&mut vr_buf)?;
    let vr_ascii = std::str::from_utf8(&vr_buf)?;

    let mut length_buf = [0u8; VALUE_LENGTH];
    file.read_exact(&mut length_buf)?;

    let length = u16::from_le_bytes(length_buf) as usize;

    let mut value_buf = vec![0u8; length];
    file.read_exact(&mut value_buf)?;

    Ok(DataElementResult {
        tag: Tag {
            group: u16::from_le_bytes(group_tag),
            element: u16::from_le_bytes(element_tag),
        },
        data_element: DataElement {
            vr: Some(vr_ascii.into()),
            length,
            raw: Some(value_buf),
        },
        length: TAG_HALF_LENGTH + TAG_HALF_LENGTH + VR_LENGTH + VALUE_LENGTH + length,
    })
}
