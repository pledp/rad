use std::path::Path;
use std::io::{BufReader, Read};
use std::fs::File;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

const PREAMBLE_LENGTH: usize = 128;
const PREFIX_LENGTH: usize = 4;

const PREFIX: &str = "DICM";

struct DicomFileHeader {

}

struct DicomData {

}

pub struct DicomFile {
    metadata: DicomFileHeader,
    data: DicomData
}

pub fn open_file<P: AsRef<Path>>(path: P) -> Result<DicomFile> {
    let path = path.as_ref();
    let mut file = BufReader::new(File::open(path)?);

    let mut preamble = [0u8; PREAMBLE_LENGTH];
    file.read_exact(&mut preamble)?;
    
    let mut prefix = [0u8; PREFIX_LENGTH];
    file.read_exact(&mut prefix)?;

    let prefix = str::from_utf8(&prefix)?;

    println!("Prefix: {}", prefix);    

    Ok(DicomFile {
        metadata: DicomFileHeader {  },
        data: DicomData {  }
    })
}