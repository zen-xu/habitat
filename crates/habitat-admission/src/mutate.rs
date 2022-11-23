use axum::Json;
use habitat_api::Job;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, ResourceExt,
};
use std::error::Error;
use tracing::*;

use crate::util::try_cast_dynamic_obj_into_job;

pub async fn handler(
    Json(body): Json<AdmissionReview<DynamicObject>>,
) -> Json<AdmissionReview<DynamicObject>> {
    // Parse incoming webhook AdmissionRequest first
    let req: AdmissionRequest<_> = match body.try_into() {
        Ok(req) => req,
        Err(err) => {
            error!("invalid request: {}", err.to_string());
            return Json(AdmissionResponse::invalid(err.to_string()).into_review());
        }
    };

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        let name = obj.name_any(); // apiserver may not have generated a name yet

        res = match try_cast_dynamic_obj_into_job(&obj) {
            Ok(job) => match mutate(res.clone(), &job) {
                Ok(res) => {
                    info!("accepted: {:?} on Job {}", req.operation, name);
                    res
                }
                Err(err) => {
                    warn!("denied: {:?} on {} ({})", req.operation, name, err);
                    res.deny(err.to_string())
                }
            },
            Err(err) => {
                warn!("invalid job: {:?} on {} ({})", req.operation, name, err);
                res.deny(err)
            }
        };
    };
    // Wrap the AdmissionResponse wrapped in an AdmissionReview
    Json(res.into_review())
}

// The main handler and core business logic, failures here implies rejected applies
fn mutate(res: AdmissionResponse, _job: &Job) -> Result<AdmissionResponse, Box<dyn Error>> {
    Ok(res)
}
