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

#[cfg(test)]
mod test {
    use std::net::IpAddr;

    use super::*;
    use anyhow::Result;
    use habitat_api::batch;
    use kube::{
        api::{Api, DeleteParams, PostParams},
        client::Client,
    };
    use tokio::time::Duration;

    #[tokio::test]
    #[ignore] // needs kube config
    async fn validate_parallelism() -> Result<()> {
        let client = Client::try_default().await.unwrap();
        let ip_addr = std::env::var("ADMISSION_PRIVATE_IP")?.parse::<IpAddr>()?;
        let server_cert = std::env::var("ADMISSION_CERT")?.parse::<PathBuf>()?;
        let server_key = std::env::var("ADMISSION_KEY")?.parse::<PathBuf>()?;

        // start admission
        let fut = tokio::spawn(async move {
            run(ip_addr, 8443, server_cert, server_key).await;
        });
        tokio::time::sleep(Duration::from_secs(1)).await;

        let jobs: Api<batch::Job> = Api::default_namespaced(client.clone());

        // create valid job
        let _ = jobs.delete("valid-job", &DeleteParams::default()).await;
        let valid_job = batch::Job::new("valid-job", batch::JobSpec {
            scheduler_name: None,
            parallelism: batch::ParallelismSpec { min: 1, max: 2 },
            priority: None,
            priority_class_name: None,
            template: batch::PodTemplate {
                metadata: None,
                spec: batch::PodSpec::default(),
            },
        });
        jobs.create(&PostParams::default(), &valid_job).await?;

        // create invalid job
        let _ = jobs.delete("invalid-job", &DeleteParams::default()).await;
        let invalid_job = batch::Job::new("invalid-job", batch::JobSpec {
            scheduler_name: None,
            parallelism: batch::ParallelismSpec { min: 3, max: 2 },
            priority: None,
            priority_class_name: None,
            template: batch::PodTemplate {
                metadata: None,
                spec: batch::PodSpec::default(),
            },
        });
        match jobs.create(&PostParams::default(), &invalid_job).await {
            Ok(_) => return Err(anyhow::anyhow!("invalid job is accepted by the admission")),
            Err(e) => {
                if !e
                    .to_string()
                    .contains("parallelism.min can't greater than parallelism.max")
                {
                    return Err(anyhow::anyhow!(
                        "error is not caused by parallelism validation: {e}"
                    ));
                }
            }
        }

        fut.abort();
        assert!(fut.await.unwrap_err().is_cancelled());
        Ok(())
    }

    #[tokio::test]
    #[ignore] // needs kube config
    async fn validate_priority() -> Result<()> {
        let client = Client::try_default().await.unwrap();
        let ip_addr = std::env::var("ADMISSION_PRIVATE_IP")?.parse::<IpAddr>()?;
        let server_cert = std::env::var("ADMISSION_CERT")?.parse::<PathBuf>()?;
        let server_key = std::env::var("ADMISSION_KEY")?.parse::<PathBuf>()?;

        // start admission
        let fut = tokio::spawn(async move {
            run(ip_addr, 8443, server_cert, server_key).await;
        });
        tokio::time::sleep(Duration::from_secs(1)).await;

        let jobs: Api<batch::Job> = Api::default_namespaced(client.clone());

        // create job with priority
        let _ = jobs.delete("valid-job1", &DeleteParams::default()).await;
        let valid_job = batch::Job::new("valid-job1", batch::JobSpec {
            scheduler_name: None,
            parallelism: batch::ParallelismSpec { min: 1, max: 2 },
            priority: Some(1),
            priority_class_name: None,
            template: batch::PodTemplate {
                metadata: None,
                spec: batch::PodSpec::default(),
            },
        });
        jobs.create(&PostParams::default(), &valid_job).await?;

        // create job with priority class name
        let _ = jobs.delete("valid-job1", &DeleteParams::default()).await;
        let valid_job = batch::Job::new("valid-job1", batch::JobSpec {
            scheduler_name: None,
            parallelism: batch::ParallelismSpec { min: 1, max: 2 },
            priority: None,
            priority_class_name: Some("system-node-critical".to_string()),
            template: batch::PodTemplate {
                metadata: None,
                spec: batch::PodSpec::default(),
            },
        });
        jobs.create(&PostParams::default(), &valid_job).await?;

        // create job with both priority and priority class name
        let _ = jobs.delete("invalid-job", &DeleteParams::default()).await;
        let invalid_job = batch::Job::new("invalid-job", batch::JobSpec {
            scheduler_name: None,
            parallelism: batch::ParallelismSpec { min: 1, max: 2 },
            priority: Some(1),
            priority_class_name: Some("system-node-critical".to_string()),
            template: batch::PodTemplate {
                metadata: None,
                spec: batch::PodSpec::default(),
            },
        });
        match jobs.create(&PostParams::default(), &invalid_job).await {
            Ok(_) => return Err(anyhow::anyhow!("invalid job is accepted by the admission")),
            Err(e) => {
                if !e
                    .to_string()
                    .contains("can't specify both priority and priorityClassName")
                {
                    return Err(anyhow::anyhow!("error is not caused by priority validation: {e}"));
                }
            }
        }

        fut.abort();
        assert!(fut.await.unwrap_err().is_cancelled());
        Ok(())
    }
}
