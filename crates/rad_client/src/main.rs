use std::path::Path;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

use rad_common::open_file;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let file = open_file(Path::new("assets/sample.dcm"))?;

    let mut stream = TcpStream::connect("127.0.0.1:42069").await?;
    println!("Connected to server");

    stream.write_all(b"Hello from client!\n").await?;

    // Read response
    let mut buffer = vec![0; 1024];
    let n = stream.read(&mut buffer).await?;
    println!("Server replied: {}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}
