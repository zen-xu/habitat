use std::path::PathBuf;

use clap::Parser;
use tracing::*;
use warp::Filter;

mod mutate;
mod validate;


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

    let mutate = warp::path("mutate")
        .and(warp::body::json())
        .and_then(crate::mutate::handler)
        .with(warp::trace::request());

    let validate = warp::path("validate")
        .and(warp::body::json())
        .and_then(crate::validate::handler)
        .with(warp::trace::request());

    info!("starting habitat admission controller");
    warp::serve(warp::post().and(mutate.or(validate)))
        .tls()
        .cert_path(cert_path)
        .key_path(key_path)
        .run(addr.parse::<std::net::SocketAddr>().unwrap())
        .await;
}
