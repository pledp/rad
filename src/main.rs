use core::net::SocketAddr;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt}, 
    net::{TcpListener, TcpStream}
};

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:42069").await?;
    println!("Listening for connections...");

    loop {
        let (mut tcp, mut socket_addr) = server.accept().await?;
        tokio::spawn(handle_client(tcp, socket_addr)); 
    }

    Ok(())
}

async fn handle_client(mut tcp: TcpStream, mut socket_addr: SocketAddr) -> Result<()> {
    println!("Connected client: {}:{}", socket_addr.ip(), socket_addr.port());
    let mut buffer = [0u8; 16];

    loop {
        let n = tcp.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        let _ = tcp.write(&buffer[..n]).await?;
    }

    Ok(())
}
