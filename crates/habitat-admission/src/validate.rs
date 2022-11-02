use habitat_api::Job;
use kube::core::{
    admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
    DynamicObject, ResourceExt,
};
use std::{convert::Infallible, error::Error};
use tracing::*;
use warp::{reply, Reply};

use crate::util::try_cast_dynamic_obj_into_job;


pub async fn handler(body: AdmissionReview<DynamicObject>) -> Result<impl Reply, Infallible> {
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
        res = match try_cast_dynamic_obj_into_job(&obj) {
            Ok(job) => match validate(res.clone(), &job) {
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
    Ok(reply::json(&res.into_review()))
}

// The main handler and core business logic, failures here implies rejected applies
fn validate(res: AdmissionResponse, obj: &Job) -> Result<AdmissionResponse, Box<dyn Error>> {
    if obj.spec.tasks.is_empty() {
        return Err("no task specified".into());
    }

    // If the task parallelism.min > parallelism.max, we reject it.
    for task in obj.spec.tasks.iter() {
        if task.parallelism.min > task.parallelism.max {
            return Err(format!(
                "task `{}` parallelism.min can't greater than parallelism.max",
                task.name
            )
            .into());
        }
    }

    Ok(res)
}
