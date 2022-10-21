use tracing::*;
use warp::Filter;

mod mutate;
mod validate;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // You must generate a certificate for the service / url,
    // encode the CA in the MutatingWebhookConfiguration, and terminate TLS here.
    // See admission_setup.sh + admission_controller.yaml.tpl for how to do this.
    let addr = format!("{}:8443", std::env::var("ADMISSION_PRIVATE_IP").unwrap());

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
        .cert_path("habitat-admission/caches/admission-controller-tls.crt")
        .key_path("habitat-admission/caches/admission-controller-tls.key")
        // .run(([0, 0, 0, 0], 8443)) // in-cluster
        .run(addr.parse::<std::net::SocketAddr>().unwrap()) // local-dev
        .await;
}
