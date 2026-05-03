mod service_user;

use core::net::SocketAddr;
use std::io::Cursor;
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use tokio::{
    net::{TcpListener},
};

use tracing::{Level, info};
use tracing_subscriber::{FmtSubscriber, fmt};

use eradic::ul::event::{Indication};
use eradic_ul_tokio::handle_client;

use crate::service_user::LocalUpperLayerServiceUser;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;


#[tokio::main]
async fn main() -> Result<()> {
    tracing_log::LogTracer::init()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_file(true)
        .with_line_number(true)
        .fmt_fields(fmt::format::DefaultFields::new())
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("System initialized");

    let ul_scu = Arc::new(LocalUpperLayerServiceUser::new());

    let server = TcpListener::bind("127.0.0.1:104").await?;
    info!("Listening for connections...");

    let client_count = Arc::new(AtomicI64::new(0));

    loop {
        let (tcp, socket_addr) = server.accept().await?;
        let client_count_clone = Arc::clone(&client_count);
        let ul_scu_clone = Arc::clone(&ul_scu);

        tokio::spawn(async move {
            let scu_handler = {
                move |indication: Indication| {
                    let ul_scu_closure = Arc::clone(&ul_scu_clone);
                    async move {
                        match indication {
                            Indication::AssociateIndication(indication) => {
                                Some(ul_scu_closure.handle_associate_request(indication).await)
                            },
                            Indication::ProviderAbortIndication(indicationn) => {
                                return None
                            }
                            Indication::AbortIndication(indication) => {
                                return None;
                            }
                            _ => todo!(),
                        }
                    }
                }
            };

            client_count_clone.fetch_add(1, Ordering::AcqRel);

            let result = handle_client(tcp, socket_addr, scu_handler).await;

            client_count_clone.fetch_sub(1, Ordering::AcqRel);

            result
        });
    }
}
