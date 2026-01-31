use std::io::Cursor;
use std::path::Path;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

use rad_common::open_file;
use rad_common::pdu::DicomPdu;
use rad_common::associate::AAssociateRqAc;
use rad_common::pdu::read_dicom_pdu;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let file = open_file(Path::new("assets/sample.dcm"))?;

    let mut stream = TcpStream::connect("127.0.0.1:42069").await?;
    println!("Connected to server");

    let buffer: &[u8] = &[0x01, 0x02, 0xA5, 0xFF, 0x10, 0x00, 0xB3];

    let mut writer = BufWriter::new(stream);

    writer.write_all(buffer).await?;
    writer.flush().await?;

    // Read response
    let mut buffer = vec![0; 1024];
    let n = writer.read(&mut buffer).await?;
    println!("Server replied: {}", String::from_utf8_lossy(&buffer[..n]));

    let pdu = read_dicom_pdu(Cursor::new(buffer))?;

    if let DicomPdu::AssociateRqAc(assoc) = pdu {
        println!("{}", assoc.length);
    }

    Ok(())
}
