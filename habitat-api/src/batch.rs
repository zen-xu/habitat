use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[kube(kind = "Job", group = "batch.habitat", version = "beta1", namespaced)]
#[kube(status = "JobStatus", shortname = "hj")]
pub struct JobSpec {
    /// If specified, the pod will be dispatched by specified scheduler.
    /// If not specified, the pod will be dispatched by default scheduler.
    scheduler_name: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct JobStatus {
    phase: JobStatusPhase,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum JobStatusPhase {
    Pending,
    Aborting,
    Aborted,
    Running,
    Restarting,
    Completing,
    Terminating,
    Terminated,
    Failed,
}
