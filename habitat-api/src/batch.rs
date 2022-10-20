use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[kube(
    namespaced,
    kind = "Job",
    group = "batch.habitat",
    version = "beta1",
    shortname = "hj",
    shortname = "hjob",
    status = "JobStatus",
    printcolumn = r#"{"name": "Pending", "jsonPath": ".status.pending", "type": "integer", "priority": 1}"#,
    printcolumn = r#"{"name": "Running", "jsonPath": ".status.running", "type": "integer", "priority": 1}"#,
    printcolumn = r#"{"name": "Terminating", "jsonPath": ".status.terminating", "type": "integer", "priority": 1}"#,
    printcolumn = r#"{"name": "Succeeded", "jsonPath": ".status.succeeded", "type": "integer", "priority": 1}"#,
    printcolumn = r#"{"name": "Failed", "jsonPath": ".status.failed", "type": "integer", "priority": 1}"#,
    printcolumn = r#"{"name": "Status", "jsonPath": ".status.phase", "type": "string", "priority": 0}"#,
    printcolumn = r#"{"name": "Start_Time", "jsonPath": ".status.startTime", "type": "string", "priority": 0}"#,
    printcolumn = r#"{"name": "Completion_Time", "jsonPath": ".status.completionTime", "type": "string", "priority": 0}"#,
    printcolumn = r#"{"name": "Age", "jsonPath": ".metadata.creationTimestamp", "type": "date", "priority": 0}"#
)]
pub struct JobSpec {
    /// If specified, the pod will be dispatched by specified scheduler.
    /// If not specified, the pod will be dispatched by default scheduler.
    pub scheduler_name: Option<String>,

    /// Specifies the maximum desired number of pods the job should run at any
    /// given time.
    #[serde(default = "default_parallelism")]
    pub parallelism: u32,

    /// Specifies the minimum desired number of pods the job should run.
    #[serde(default = "default_parallelism")]
    pub min_parallelism: u32,

    /// If specified, indicates the Job's priority.
    pub priority_class_name: Option<String>,

    /// The priority value.
    ///
    /// Note that only one of `priority_class_name` or `priority` can be
    /// specified.
    pub priority: Option<u32>,

    /// The pod template.
    pub template: PodTemplate,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub struct JobStatus {
    /// Job status phase.
    pub phase: JobStatusPhase,

    /// Represents the time when the job status phase became `Running`. It is represented in RFC3339 form
    /// and is in UTC.
    pub start_time: Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,

    /// Represents the time when the job was completed (). It is represented in RFC3339 form and is in UTC.
    pub completion_time: Option<k8s_openapi::apimachinery::pkg::apis::meta::v1::Time>,

    /// The number of pods which reached phase `Pending`.
    pub pending: Option<u32>,

    /// The number of pods which reached phase `Running`.
    pub running: Option<u32>,

    /// The number of pods which reached phase `Terminating`.
    pub terminating: Option<u32>,

    /// The number of pods which reached phase `Succeeded`.
    pub succeeded: Option<u32>,

    /// The number of pods which reached phase `Failed`.    
    pub failed: Option<u32>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum JobStatusPhase {
    // Pending means the job has been accepted by the system, but one or more of pods has not been
    // scheduled.
    Pending,
    // Running means that if the job contains any `Running` pod, its status will be `Running`.
    Running,
    // Terminating means that the job is terminated, and waiting for releasing pods.
    Terminating,
    // Succeeded means that the job is completed with success.
    Succeeded,
    // Succeeded means that the job is completed with failure.
    Failed,
    // Terminated means that the job is completed with unexpected.
    Terminated,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct PodTemplate {
    /// Pod metadata
    pub metadata: Option<PodMeta>,

    /// Specification of the desired behavior of the pod.
    pub spec: PodSpec,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct PodMeta {
    /// Annotations is an unstructured key value map stored with a resource that may be set by
    /// external tools to store and retrieve arbitrary metadata. They are not queryable and should
    /// be preserved when modifying objects. More info: http://kubernetes.io/docs/user-guide/annotations
    pub annotations: Option<std::collections::BTreeMap<String, String>>,

    /// Map of string keys and values that can be used to organize and categorize (scope and select)
    /// objects. May match selectors of replication controllers and services.
    /// More info: http://kubernetes.io/docs/user-guide/labels
    pub labels: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct PodSpec {
    /// Optional duration in seconds the pod may be active on the node relative
    /// to StartTime before the system will actively try to mark it failed and
    /// kill associated containers. Value must be a positive integer.
    // pub active_deadline_seconds: Option<i64>,

    /// If specified, the pod's scheduling constraints
    // pub affinity: Option<k8s_openapi::api::core::v1::Affinity>,

    /// AutomountServiceAccountToken indicates whether a service account token
    /// should be automatically mounted.
    // pub automount_service_account_token: Option<bool>,

    /// List of containers belonging to the pod. Containers cannot currently be
    /// added or removed. There must be at least one container in a Pod. Cannot
    /// be updated.
    pub containers: Vec<k8s_openapi::api::core::v1::Container>,

    /// Specifies the DNS parameters of a pod. Parameters specified here will
    /// be merged to the generated DNS configuration based on DNSPolicy.
    // pub dns_config: Option<k8s_openapi::api::core::v1::PodDNSConfig>,

    /// Set DNS policy for the pod. Defaults to "ClusterFirst". Valid values
    /// are 'ClusterFirstWithHostNet', 'ClusterFirst', 'Default' or 'None'. DNS
    /// parameters given in DNSConfig will be merged with the policy selected
    /// with DNSPolicy. To have DNS options set along with hostNetwork, you
    /// have to specify DNS policy explicitly to 'ClusterFirstWithHostNet'.
    // pub dns_policy: Option<String>,

    /// EnableServiceLinks indicates whether information about services should
    /// be injected into pod's environment variables, matching the syntax of
    /// Docker links. Optional: Defaults to true.
    // pub enable_service_links: Option<bool>,

    /// List of ephemeral containers run in this pod. Ephemeral containers may
    /// be run in an existing pod to perform user-initiated actions such as
    /// debugging. This list cannot be specified when creating a pod, and it
    /// cannot be modified by updating the pod spec. In order to add an
    /// ephemeral container to an existing pod, use the pod's
    /// ephemeralcontainers subresource. This field is beta-level and available
    /// on clusters that haven't disabled the EphemeralContainers feature gate.
    // pub ephemeral_containers:
    // Option<Vec<k8s_openapi::api::core::v1::EphemeralContainer>>,

    /// HostAliases is an optional list of hosts and IPs that will be injected
    /// into the pod's hosts file if specified. This is only valid for
    /// non-hostNetwork pods.
    // pub host_aliases: Option<Vec<k8s_openapi::api::core::v1::HostAlias>>,

    /// Use the host's ipc namespace. Optional: Default to false.
    // pub host_ipc: Option<bool>,

    /// Host networking requested for this pod. Use the host's network
    /// namespace. If this option is set, the ports that will be used must be
    /// specified. Default to false.
    // pub host_network: Option<bool>,

    /// Use the host's pid namespace. Optional: Default to false.
    // pub host_pid: Option<bool>,

    /// Specifies the hostname of the Pod If not specified, the pod's hostname
    /// will be set to a system-defined value.
    // pub hostname: Option<String>,

    /// ImagePullSecrets is an optional list of references to secrets in the same namespace
    /// to use for pulling any of the images used by this PodSpec. If specified, these secrets
    /// will be passed to individual puller implementations for them to use.
    /// More info: https://kubernetes.io/docs/concepts/containers/images#specifying-imagepullsecrets-on-a-pod
    pub image_pull_secrets: Option<Vec<k8s_openapi::api::core::v1::LocalObjectReference>>,

    /// List of initialization containers belonging to the pod. Init containers are executed in
    /// order prior to containers being started. If any init container fails, the pod is considered
    /// to have failed and is handled according to its restartPolicy. The name for an init container
    /// or normal container must be unique among all containers. Init containers may not have
    /// Lifecycle actions, Readiness probes, Liveness probes, or Startup probes. The
    /// resourceRequirements of an init container are taken into account during scheduling
    /// by finding the highest request/limit for each resource type, and then using the max of
    /// of that value or the sum of the normal containers. Limits are applied to init containers in
    /// a similar fashion. Init containers cannot currently be added or removed. Cannot be updated.
    /// More info: https://kubernetes.io/docs/concepts/workloads/pods/init-containers/
    pub init_containers: Option<Vec<k8s_openapi::api::core::v1::Container>>,

    /// NodeName is a request to schedule this pod onto a specific node. If it
    /// is non-empty, the scheduler simply schedules this pod onto that node,
    /// assuming that it fits resource requirements.
    // pub node_name: Option<String>,

    /// NodeSelector is a selector which must be true for the pod to fit on a node. Selector which
    /// must match a node's labels for the pod to be scheduled on that node.
    /// More info: https://kubernetes.io/docs/concepts/configuration/assign-pod-node/
    // pub node_selector: Option<std::collections::BTreeMap<String, String>>,

    /// Specifies the OS of the containers in the pod. Some pod and container
    /// fields are restricted if this is set.
    ///
    /// If the OS field is set to linux, the following fields must be unset:
    /// -securityContext.windowsOptions
    ///
    /// If the OS field is set to windows, following fields must be unset: -
    /// spec.hostPID - spec.hostIPC - spec.securityContext.seLinuxOptions -
    /// spec.securityContext.seccompProfile - spec.securityContext.fsGroup -
    /// spec.securityContext.fsGroupChangePolicy - spec.securityContext.sysctls
    /// - spec.shareProcessNamespace - spec.securityContext.runAsUser -
    /// spec.securityContext.runAsGroup -
    /// spec.securityContext.supplementalGroups -
    /// spec.containers\[*\].securityContext.seLinuxOptions -
    /// spec.containers\[*\].securityContext.seccompProfile -
    /// spec.containers\[*\].securityContext.capabilities -
    /// spec.containers\[*\].securityContext.readOnlyRootFilesystem -
    /// spec.containers\[*\].securityContext.privileged -
    /// spec.containers\[*\].securityContext.allowPrivilegeEscalation -
    /// spec.containers\[*\].securityContext.procMount -
    /// spec.containers\[*\].securityContext.runAsUser -
    /// spec.containers\[*\].securityContext.runAsGroup This is a beta field
    /// and requires the IdentifyPodOS feature
    // pub os: Option<k8s_openapi::api::core::v1::PodOS>,

    /// Overhead represents the resource overhead associated with running a pod for a given
    /// RuntimeClass. This field will be autopopulated at admission time by the RuntimeClass
    /// admission controller. If the RuntimeClass admission controller is enabled, overhead must
    /// not be set in Pod create requests. The RuntimeClass admission controller will reject Pod
    /// create requests which have the overhead already set. If RuntimeClass is configured and
    /// selected in the PodSpec, Overhead will be set to the value defined in the corresponding
    /// RuntimeClass, otherwise it will remain unset and treated as zero.
    /// More info: https://git.k8s.io/enhancements/keps/sig-node/688-pod-overhead/README.md
    // pub overhead: Option<
    //    std::collections::BTreeMap<String,
    // k8s_openapi::apimachinery::pkg::api::resource::Quantity>, >,

    /// PreemptionPolicy is the Policy for preempting pods with lower priority.
    /// One of Never, PreemptLowerPriority. Defaults to PreemptLowerPriority if
    /// unset.
    // pub preemption_policy: Option<String>,

    /// The priority value. Various system components use this field to find
    /// the priority of the pod. When Priority Admission Controller is enabled,
    /// it prevents users from setting this field. The admission controller
    /// populates this field from PriorityClassName. The higher the value, the
    /// higher the priority.
    // pub priority: Option<i32>,

    /// If specified, indicates the pod's priority. "system-node-critical" and
    /// "system-cluster-critical" are two special keywords which indicate the
    /// highest priorities with the former being the highest priority. Any
    /// other name must be defined by creating a PriorityClass object with that
    /// name. If not specified, the pod priority will be default or zero if
    /// there is no default.
    // pub priority_class_name: Option<String>,

    /// If specified, all readiness gates will be evaluated for pod readiness. A pod is ready when
    /// all its containers are ready AND all conditions specified in the readiness gates have
    /// status equal to "True" More info: https://git.k8s.io/enhancements/keps/sig-network/580-pod-readiness-gates
    // pub readiness_gates:
    // Option<Vec<k8s_openapi::api::core::v1::PodReadinessGate>>,

    /// Restart policy for all containers within the pod. One of Always, OnFailure, Never.
    /// Default to Always. More info: https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/#restart-policy
    pub restart_policy: Option<String>,

    /// RuntimeClassName refers to a RuntimeClass object in the node.k8s.io group, which should be
    /// used to run this pod.  If no RuntimeClass resource matches the named class, the pod will
    /// not be run. If unset or empty, the "legacy" RuntimeClass will be used, which is an implicit
    /// class with an empty definition that uses the default runtime handler.
    /// More info: https://git.k8s.io/enhancements/keps/sig-node/585-runtime-class
    // pub runtime_class_name: Option<String>,

    /// If specified, the pod will be dispatched by specified scheduler. If not
    /// specified, the pod will be dispatched by default scheduler.
    // pub scheduler_name: Option<String>,

    /// SecurityContext holds pod-level security attributes and common
    /// container settings. Optional: Defaults to empty.  See type description
    /// for default values of each field.
    pub security_context: Option<k8s_openapi::api::core::v1::PodSecurityContext>,

    /// DeprecatedServiceAccount is a depreciated alias for ServiceAccountName.
    /// Deprecated: Use serviceAccountName instead.
    pub service_account: Option<String>,

    /// ServiceAccountName is the name of the ServiceAccount to use to run this pod.
    /// More info: https://kubernetes.io/docs/tasks/configure-pod-container/configure-service-account/
    pub service_account_name: Option<String>,

    /// If true the pod's hostname will be configured as the pod's FQDN, rather
    /// than the leaf name (the default). In Linux containers, this means
    /// setting the FQDN in the hostname field of the kernel (the nodename
    /// field of struct utsname). In Windows containers, this means setting the
    /// registry value of hostname for the registry key
    /// HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\
    /// Parameters to FQDN. If a pod does not have FQDN, this has no effect.
    /// Default to false.
    pub set_hostname_as_fqdn: Option<bool>,

    /// Share a single process namespace between all of the containers in a
    /// pod. When this is set containers will be able to view and signal
    /// processes from other containers in the same pod, and the first process
    /// in each container will not be assigned PID 1. HostPID and
    /// ShareProcessNamespace cannot both be set. Optional: Default to false.
    pub share_process_namespace: Option<bool>,

    /// If specified, the fully qualified Pod hostname will be
    /// "\<hostname\>.\<subdomain\>.\<pod namespace\>.svc.\<cluster domain\>".
    /// If not specified, the pod will not have a domainname at all.
    pub subdomain: Option<String>,

    /// Optional duration in seconds the pod needs to terminate gracefully. May
    /// be decreased in delete request. Value must be non-negative integer. The
    /// value zero indicates stop immediately via the kill signal (no
    /// opportunity to shut down). If this value is nil, the default grace
    /// period will be used instead. The grace period is the duration in
    /// seconds after the processes running in the pod are sent a termination
    /// signal and the time when the processes are forcibly halted with a kill
    /// signal. Set this value longer than the expected cleanup time for your
    /// process. Defaults to 30 seconds.
    pub termination_grace_period_seconds: Option<i64>,

    /// If specified, the pod's tolerations.
    // pub tolerations: Option<Vec<k8s_openapi::api::core::v1::Toleration>>,

    /// TopologySpreadConstraints describes how a group of pods ought to spread
    /// across topology domains. Scheduler will schedule pods in a way which
    /// abides by the constraints. All topologySpreadConstraints are ANDed.
    // pub topology_spread_constraints:
    //    Option<Vec<k8s_openapi::api::core::v1::TopologySpreadConstraint>>,

    /// List of volumes that can be mounted by containers belonging to the pod.
    /// More info: https://kubernetes.io/docs/concepts/storage/volumes
    pub volumes: Option<Vec<k8s_openapi::api::core::v1::Volume>>,
}

fn default_parallelism() -> u32 { 1 }
