mod service_user;

use core::net::SocketAddr;
use std::io::Cursor;
use std::net::IpAddr;
use std::string::String;
use std::sync::{
    Arc,
    atomic::{AtomicI64, Ordering},
};

use tokio::net::TcpListener;

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _; // brings .tracer() into scope
use opentelemetry_otlp::SpanExporter;
use opentelemetry_sdk::trace::SdkTracerProvider;
use tokio::sync::mpsc;
use tracing::{Level, info};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

use eradic::ul::event::{Request, ServiceProviderToServiceUser, ServiceUserToServiceProvider};
use eradic_ul_tokio::{acceptor_handle_client};

use crate::service_user::LocalUpperLayerServiceUser;

pub type Result<T> = std::result::Result<T, Error>;
pub type Error = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<()> {
    let provider = init_telemetry()?;

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
            let (scu_tx, scu_rx) = mpsc::channel::<>(32);

            let scu_handler = {
                move |indication: ServiceProviderToServiceUser| {
                    let ul_scu_closure = Arc::clone(&ul_scu_clone);
                    let scu_tx_clone = scu_tx.clone();
                    async move {
                        match indication {
                            ServiceProviderToServiceUser::AssociateIndication(indication) => {
                                scu_tx_clone.send(
                                    ServiceUserToServiceProvider::Event(ul_scu_closure.handle_associate_request(indication).await)
                                ).await;
                            }
                            _ => {}
                        }
                    }
                }
            };

            client_count_clone.fetch_add(1, Ordering::AcqRel);

            let result = acceptor_handle_client(tcp, socket_addr, scu_handler, scu_rx).await;

            client_count_clone.fetch_sub(1, Ordering::AcqRel);

            result
        });
    }
}

pub fn init_telemetry() -> Result<SdkTracerProvider> {
    // --- traces -> Tempo ---
    let exporter = SpanExporter::builder()
        .with_tonic()
        .build()
        .expect("Failed to build span exporter");

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build();

    global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("eradic_ul");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    metrics_exporter_prometheus::PrometheusBuilder::new()
        .install()
        .expect("Failed to install Prometheus exporter");

    let (loki_layer, task) = tracing_loki::builder()
        .label("service_name", "eradic_ul")?
        .build_url(tracing_loki::url::Url::parse("http://localhost:3100")?)?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            "off,server=debug,eradic_ul=debug"
        ))
        .with(
            fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .fmt_fields(fmt::format::DefaultFields::new()),
        )
        .with(otel_layer)
        .with(loki_layer)
        .init();

    tokio::spawn(task);

    Ok(provider)
}
