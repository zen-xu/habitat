use std::path::PathBuf;

use axum::{
    routing::{get, post},
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, help = "Server addr", default_value = "0.0.0.0")]
    ip_addr: std::net::IpAddr,

    #[arg(long, help = "Server port", default_value_t = 8443)]
    port: u32,

    #[arg(long, required = true, help = "Specify the file path to read the certificate")]
    cert_path: PathBuf,

    #[arg(long, required = true, help = "Specify the file path to read the private key")]
    key_path: PathBuf,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    run(args.ip_addr, args.port, args.cert_path, args.key_path).await;
}

async fn run(ip_addr: std::net::IpAddr, port: u32, cert_path: PathBuf, key_path: PathBuf) {
    let addr = format!("{}:{}", ip_addr, port);

    let app = Router::new()
        .route("/validate", post(habitat_admission::validate::handler))
        .route("/mutate", post(habitat_admission::mutate::handler))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        // Reminder: routes added *after* TraceLayer are not subject to its logging behavior
        .route("/health", get(|| async { "healthy" }));

    let config = RustlsConfig::from_pem_file(cert_path, key_path).await.unwrap();
    axum_server::bind_rustls(addr.parse().unwrap(), config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
