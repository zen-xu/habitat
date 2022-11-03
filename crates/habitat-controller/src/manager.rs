use std::{sync::Arc, time::Duration};

use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::{future::BoxFuture, FutureExt, StreamExt};
use habitat_api::{batch::JobStatus, Job};
use kube::{
    api::{Api, ListParams, Patch, PatchParams},
    client::Client,
    runtime::{
        controller::{Action, Controller},
        events::{Event, EventType, Recorder, Reporter},
        finalizer::{finalizer, Event as Finalizer},
    },
    Resource, ResourceExt,
};
use serde::Serialize;
use tokio::sync::RwLock;
use tracing::{info, warn};

static JOB_FINALIZER: &str = "job.batch.habitat";

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

        let jobs = Api::<Job>::all(client);

        // Ensure CRD is installed before loop-watching
        let _ = jobs
            .list(&ListParams::default().limit(1))
            .await
            .expect("is habitat installed?");

        // All good. Start controller and return its future.
        let controller = Controller::new(jobs, ListParams::default())
            .shutdown_on_signal()
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

    finalizer(&jobs, JOB_FINALIZER, job, |event| async {
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
        let client = ctx.client.clone();
        let reporter = ctx.diagnostics.read().await.reporter.clone();
        let recorder = Recorder::new(client.clone(), reporter, self.object_ref(&()));

        let name = self.name_any();
        let ns = self.namespace().unwrap();
        let jobs: Api<Job> = Api::namespaced(client, &ns);

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
