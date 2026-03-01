use std::path::Path;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

use rad_common::associate::{AssociateRqAcPdu, serialize_association_pdu};
use rad_common::open_file;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let file = open_file(Path::new("assets/sample.dcm"))?;

    let mut stream = TcpStream::connect("127.0.0.1:104").await?;
    println!("Connected to server");

    let pdu = AssociateRqAcPdu::new_rq("test1", "rad");

    let mut writer = BufWriter::new(stream);

    writer
        .write_all(serialize_association_pdu(&pdu)?.as_slice())
        .await?;
    writer.flush().await?;

    // Read response
    let mut buffer = vec![0; 1024];
    let n = writer.read(&mut buffer).await?;
    println!("Server replied: {}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}
