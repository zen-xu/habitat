use std::{collections::HashMap, sync::Arc};

use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{future::BoxFuture, FutureExt, StreamExt};
use habitat_api::{
    batch::{JobStatus, JobStatusPhase},
    Job,
};
use k8s_openapi::api::core::v1::{Pod, PodSpec};
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
    client::Client,
    core::ObjectMeta,
    runtime::{
        controller::{Action, Controller},
        events::{Event, EventType, Recorder, Reporter},
        finalizer::{finalizer, Event as Finalizer},
    },
    Resource, ResourceExt,
};
use serde::Serialize;
use tokio::{sync::RwLock, time::Duration};
use tracing::{info, warn};

static FINALIZER_NAME: &str = "controller.batch.habitat";
static TASK_OWNER_LABEL: &str = "habitat-task-owner";
static TASK_NAME_LABEL: &str = "habitat-task";

// Context for our reconciler
#[derive(Clone)]
pub struct Context {
    /// Kubernetes client
    client: Client,
    /// Diagnostics read by the web server
    diagnostics: Arc<RwLock<Diagnostics>>,
}


/// Diagnostics to be exposed by the web server
#[derive(Clone, Serialize)]
pub struct Diagnostics {
    #[serde(deserialize_with = "from_ts")]
    pub last_event: DateTime<Utc>,
    #[serde(skip)]
    pub reporter: Reporter,
}

impl Diagnostics {
    fn new() -> Self {
        Self {
            last_event: Utc::now(),
            reporter: "habitat-controller".into(),
        }
    }
}


#[derive(Clone)]
pub struct Manager {
    /// Diagnostics populated by the reconciler
    diagnostics: Arc<RwLock<Diagnostics>>,
}

impl Manager {
    pub async fn new(client: Client) -> (Self, BoxFuture<'static, ()>) {
        let diagnostics = Arc::new(RwLock::new(Diagnostics::new()));
        let context = Arc::new(Context {
            client: client.clone(),
            diagnostics: diagnostics.clone(),
        });

        let pods = Api::<Pod>::all(client.clone());
        let jobs = Api::<Job>::all(client);

        // Ensure CRD is installed before loop-watching
        let _ = jobs
            .list(&ListParams::default().limit(1))
            .await
            .expect("is habitat installed?");

        // All good. Start controller and return its future.
        let controller = Controller::new(jobs, ListParams::default())
            .shutdown_on_signal()
            .owns(pods, ListParams::default())
            .run(reconciler, error_policy, context)
            .filter_map(|x| async move { std::result::Result::ok(x) })
            .for_each(|_| futures::future::ready(()))
            .boxed();

        (Self { diagnostics }, controller)
    }

    /// State getter
    pub async fn diagnostics(&self) -> Diagnostics { self.diagnostics.read().await.clone() }
}


async fn reconciler(job: Arc<Job>, ctx: Arc<Context>) -> Result<Action> {
    let client = ctx.client.clone();
    let ns = job.namespace().unwrap();
    let jobs: Api<Job> = Api::namespaced(client, &ns);

    finalizer(&jobs, FINALIZER_NAME, job, |event| async {
        match event {
            Finalizer::Apply(job) => job.reconcile(ctx.clone()).await,
            Finalizer::Cleanup(job) => job.cleanup(ctx.clone()).await,
        }
    })
    .await
    .map_err(Error::FinalizerError)
}

fn error_policy(job: Arc<Job>, error: &Error, ctx: Arc<Context>) -> Action { job.error_policy(error, ctx) }

#[async_trait]
pub trait Reconciler: Resource {
    async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, kube::Error>;
    async fn cleanup(&self, ctx: Arc<Context>) -> Result<Action, kube::Error>;
    fn error_policy(&self, error: &Error, ctx: Arc<Context>) -> Action;
}

#[async_trait]
impl Reconciler for Job {
    async fn reconcile(&self, ctx: Arc<Context>) -> Result<Action, kube::Error> {
        info!("reconcile");
        if let Some(status) = &self.status {
            if matches!(
                status.phase,
                JobStatusPhase::Succeeded | JobStatusPhase::Failed | JobStatusPhase::Terminated
            ) {
                return Ok(Action::await_change());
            }
        }

        let client = ctx.client.clone();
        let reporter = ctx.diagnostics.read().await.reporter.clone();
        let recorder = Recorder::new(client.clone(), reporter, self.object_ref(&()));

        let name = self.name_any();
        let ns = self.namespace().unwrap();
        let jobs: Api<Job> = Api::namespaced(client.clone(), &ns);
        let pods: Api<Pod> = Api::namespaced(client.clone(), &ns);

        if self.status.is_none() {
            recorder
                .publish(Event {
                    type_: EventType::Normal,
                    reason: "CreateJob".into(),
                    note: Some(format!("Creating Job `{}`", name)),
                    action: "Reconciling".into(),
                    secondary: None,
                })
                .await?;

            let new_status = Patch::Merge(serde_json::json!({ "status": JobStatus::default() }));
            let _ = jobs
                .patch_status(&name, &PatchParams::default(), &new_status)
                .await?;
        }

        let owned_pods = pods
            .list(&ListParams::default().labels(&new_owned_label(self)))
            .await?
            .into_iter()
            .map(|pod| (pod.name_any(), pod))
            .collect::<HashMap<_, _>>();

        for pod in build_min_owned_pods(self) {
            if !owned_pods.contains_key(&pod.name_any()) {
                // create pod
                pods.create(&PostParams::default(), &pod).await?;
                info!("created pod {}/{}", ns, pod.name_any());
            }
        }

        let mut pending = 0;
        let mut running = 0;
        let mut succeeded = 0;
        let mut failed = 0;
        let mut terminating = 0;
        for pod in owned_pods.values() {
            if pod.meta().deletion_timestamp.is_some() {
                terminating += 1
            } else if let Some(pod_phase) = pod.status.as_ref().and_then(|status| status.phase.clone()) {
                match &pod_phase[..] {
                    "Pending" => pending += 1,
                    "Running" => running += 1,
                    "Succeeded" => succeeded += 1,
                    "Failed" => failed += 1,
                    _ => (),
                }
            }

            if let Some(replicas_id) = pod.name_any().split('-').last() {
                if let (Ok(replicas_id), Some(task_name)) =
                    (replicas_id.parse::<u32>(), pod.labels().get(TASK_NAME_LABEL))
                {
                    for task_spec in self.spec.tasks.iter() {
                        if task_spec.name == *task_name {
                            if replicas_id >= task_spec.parallelism.max {
                                // reclaim this pod
                                info!(
                                    "task '{}' max parallelism is {}, so reclaim pod <{}/{}>",
                                    task_name,
                                    task_spec.parallelism.max,
                                    ns,
                                    pod.name_any()
                                );
                                pods.delete(&pod.name_any(), &DeleteParams::default()).await?;
                                terminating += 1;
                            }
                            break;
                        }
                    }
                }
            }
        }
        let phase = match (pending, running, succeeded, failed, terminating) {
            (_, running, _, _, _) if running > 0 => Some(JobStatusPhase::Running),
            (0, 0, succeeded, 0, 0) if succeeded > 0 => Some(JobStatusPhase::Succeeded),
            (0, 0, _, failed, _) if failed > 0 => Some(JobStatusPhase::Failed),
            (pending, _, _, _, _) if pending > 0 => Some(JobStatusPhase::Pending),
            _ => None,
        };

        let patch = Patch::Merge({
            if let Some(phase) = phase {
                serde_json::json!({"status": {"pending": pending, "running": running, "succeeded": succeeded, "failed": failed, "terminating": terminating, "phase": phase}})
            } else {
                serde_json::json!({"status": {"pending": pending, "running": running, "succeeded": succeeded, "failed": failed, "terminating": terminating}})
            }
        });
        jobs.patch_status(&name, &PatchParams::default(), &patch).await?;
        Ok(Action::await_change())
    }

    async fn cleanup(&self, _ctx: Arc<Context>) -> Result<Action, kube::Error> {
        info!("delete job");

        Ok(Action::await_change())
    }

    fn error_policy(&self, error: &Error, _ctx: Arc<Context>) -> Action {
        warn!("reconcile failed: {:?}", error);
        Action::requeue(Duration::from_secs(5 * 60))
    }
}

fn new_owned_label(job: &Job) -> String { format!("{}={}", TASK_OWNER_LABEL, job.name_any()) }

fn build_min_owned_pods(job: &Job) -> Vec<Pod> {
    let mut pods = vec![];
    let oref = job.controller_owner_ref(&()).unwrap();
    for task in &job.spec.tasks {
        let pod_spec: PodSpec =
            serde_json::from_str(&serde_json::to_string(&task.template.spec).unwrap()).unwrap();

        for i in 0..task.parallelism.min {
            let annotations = task
                .template
                .metadata
                .as_ref()
                .and_then(|d| d.annotations.clone());
            let mut labels = task
                .template
                .metadata
                .as_ref()
                .and_then(|d| d.labels.clone())
                .unwrap_or_default();
            labels.insert(TASK_OWNER_LABEL.to_string(), job.name_any());
            labels.insert(TASK_NAME_LABEL.to_string(), task.name.clone());
            let name = format!("{}-{}", task.name, i);

            let pod = Pod {
                metadata: ObjectMeta {
                    name: Some(name),
                    owner_references: Some(vec![oref.clone()]),
                    labels: Some(labels),
                    annotations,
                    ..Default::default()
                },
                spec: Some(pod_spec.clone()),
                ..Default::default()
            };
            pods.push(pod);
        }
    }

    pods
}
