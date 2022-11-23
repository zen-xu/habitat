use habitat_api::{batch::TaskSpec, Job};
use k8s_openapi::api::core::v1::Pod;
use kube::{
    core::{
        admission::{AdmissionRequest, AdmissionResponse, AdmissionReview},
        params::PostParams,
        DynamicObject, ObjectMeta, ResourceExt,
    },
    Api, Client,
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
    let client = kube::Client::try_default().await.unwrap();

    // Then construct a AdmissionResponse
    let mut res = AdmissionResponse::from(&req);
    // req.Object always exists for us, but could be None if extending to DELETE events
    if let Some(obj) = req.object {
        let name = obj.name_any(); // apiserver may not have generated a name yet
        res = match try_cast_dynamic_obj_into_job(&obj) {
            Ok(job) => match validate(res.clone(), &job, client).await {
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
async fn validate(
    res: AdmissionResponse,
    obj: &Job,
    client: Client,
) -> Result<AdmissionResponse, Box<dyn Error>> {
    if obj.spec.tasks.is_empty() {
        return Err("no task specified".into());
    }

    let pods: Api<Pod> = Api::namespaced(client, &obj.namespace().unwrap());

    // If the task parallelism.min > parallelism.max, we reject it.
    for (idx, task) in obj.spec.tasks.iter().enumerate() {
        if task.parallelism.min > task.parallelism.max {
            return Err(format!(
                "task `{}` parallelism.min can't greater than parallelism.max",
                task.name
            )
            .into());
        }

        // create a template pod and validate it in the server side
        let pod = new_template_pod(task);
        if let Err(err) = pods
            .create(
                &PostParams {
                    dry_run: true,
                    field_manager: None,
                },
                &pod,
            )
            .await
        {
            return match err {
                kube::Error::Api(err) => {
                    let message = format!(
                        "spec.tasks[{}].template.{}",
                        idx,
                        err.message
                            .trim_start_matches(&format!("Pod \"{}\" is invalid: ", pod.name_any()))
                    );
                    Err(message.into())
                }
                _ => Err(err.into()),
            };
        }
    }

    Ok(res)
}

fn new_template_pod(task_spec: &TaskSpec) -> Pod {
    Pod {
        metadata: ObjectMeta {
            name: Some(task_spec.name.clone()),
            labels: task_spec
                .template
                .metadata
                .as_ref()
                .and_then(|m| m.labels.clone()),
            annotations: task_spec
                .template
                .metadata
                .as_ref()
                .and_then(|m| m.annotations.clone()),
            ..Default::default()
        },
        spec: Some(serde_json::from_str(&serde_json::to_string(&task_spec.template.spec).unwrap()).unwrap()),
        ..Default::default()
    }
}
