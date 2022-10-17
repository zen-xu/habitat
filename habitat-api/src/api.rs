use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const K8S_DEFAULT_SCHEDULER: &str = "default-scheduler";

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[kube(kind = "Job", group = "habitat", version = "beta1", namespaced)]
#[kube(status = "JobStatus", shortname = "hj")]
pub struct JobSpec {
    #[serde(default = "default_scheduler")]
    scheduler: String,
    queue: Option<String>,
    ttl_seconds_after_finished: Option<u32>,
}

fn default_scheduler() -> String {
    K8S_DEFAULT_SCHEDULER.into()
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
