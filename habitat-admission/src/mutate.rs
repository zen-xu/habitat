use habitat_api::Job;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    ResourceExt,
};
use std::{convert::Infallible, error::Error};
use tracing::*;
use warp::{reply, Reply};


pub async fn handler(body: AdmissionReview<Job>) -> Result<impl Reply, Infallible> {
    // Parse incoming webhook AdmissionRequest first
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Ok(reply::json(
                &AdmissionResponse::invalid(err.to_string()).into_review(),
            ));
        }
    };

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        let name = obj.name_any(); // apiserver may not have generated a name yet
        res = match mutate(res.clone(), &obj) {
            Ok(res) => {
                info!("accepted: {:?} on Job {}", req.operation, name);
                res
            }
            Err(err) => {
                warn!("denied: {:?} on {} ({})", req.operation, name, err);
                res.deny(err.to_string())
            }
        };
    };
    // Wrap the AdmissionResponse wrapped in an AdmissionReview
    Ok(reply::json(&res.into_review()))
}

// The main handler and core business logic, failures here implies rejected applies
fn mutate(res: AdmissionResponse, _job: &Job) -> Result<AdmissionResponse, Box<dyn Error>> { Ok(res) }