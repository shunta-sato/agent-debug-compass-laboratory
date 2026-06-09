use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POLICY_VERSION: &str = "default-lab-policy-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorKind {
    Agent,
    Human,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub kind: ActorKind,
    pub id: String,
}

impl Actor {
    pub fn codex() -> Self {
        Self {
            kind: ActorKind::Agent,
            id: "codex".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskTier {
    Tier0ReadOnlyObservation,
    Tier1LowRiskReversibleNonRoot,
    Tier2PrivilegedReversible,
    Tier3HardToRestore,
    Tier4NormallyProhibited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Observation,
    Control,
    Load,
    Probe,
    ReportNormalizer,
    Restore,
    HealthCheck,
    ObservationControl,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeLevel {
    None,
    User,
    SudoHelper,
    Root,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QualificationStatus {
    Builtin,
    NeedsControlTest,
    ExternalUnqualified,
    AgentCreatedUnqualified,
    Qualified,
    Refused,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolSource {
    Builtin,
    External,
    AgentCreated,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SoftwareStack {
    pub os: String,
    pub kernel: String,
    pub arch: String,
    pub board: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HardwareInventory {
    pub cpu_count: usize,
    pub memory_total_kb: Option<u64>,
    pub thermal_zones: usize,
    pub cpufreq_policies: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlSurfaceSummary {
    pub surface_id: String,
    pub available: bool,
    pub requires_privilege: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetInventory {
    pub schema_version: String,
    pub target_id: String,
    pub target: String,
    pub collected_by: String,
    pub time_unix_ms: u64,
    pub software_stack: SoftwareStack,
    pub hardware: HardwareInventory,
    pub control_surfaces: Vec<ControlSurfaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolInfo {
    pub tool_id: String,
    pub category: ToolCategory,
    pub available: bool,
    pub privilege: PrivilegeLevel,
    pub qualification: QualificationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolchainInventory {
    pub schema_version: String,
    pub target_id: String,
    pub software_stack: SoftwareStack,
    pub tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlSurfaceInventory {
    pub schema_version: String,
    pub target_id: String,
    pub surfaces: Vec<ControlSurfaceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperationBounds {
    pub duration_seconds_max: u64,
    pub thermal_celsius_abort: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CpufreqDesiredState {
    pub governor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlOperation {
    pub operation_id: String,
    pub desired_state: CpufreqDesiredState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ControlPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub run_id: String,
    pub target_id: String,
    pub risk_tier: RiskTier,
    pub approval_required: bool,
    pub restore_required: bool,
    pub operation: ControlOperation,
    pub bounds: OperationBounds,
    pub created_by: Actor,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalBounds {
    pub duration_seconds_max: u64,
    pub thermal_celsius_abort: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalRecord {
    pub schema_version: String,
    pub approval_id: String,
    pub target_id: String,
    pub risk_tier: RiskTier,
    pub operation_summary: String,
    pub approved_plan_id: String,
    pub approved_plan_digest: String,
    pub approved_operation: ControlOperation,
    pub approved_by: Actor,
    pub bounds: ApprovalBounds,
    pub restore_required: bool,
    pub approved_actions: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CpufreqPolicyState {
    pub policy: String,
    pub governor: String,
    pub scaling_min_freq: Option<String>,
    pub scaling_max_freq: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapturedState {
    pub cpufreq_policies: Vec<CpufreqPolicyState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AppliedState {
    pub governor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreStatus {
    Pending,
    Restored,
    Failed,
    DryRun,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RestoreAttemptStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RestoreAttempt {
    pub status: RestoreAttemptStatus,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RestoreLease {
    pub schema_version: String,
    pub lease_id: String,
    pub target_id: String,
    pub operation_id: String,
    pub captured_state: CapturedState,
    pub applied_state: AppliedState,
    pub restore_required: bool,
    pub restore_status: RestoreStatus,
    pub created_by_plan: String,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ControlResultStatus {
    Refused,
    DryRunOk,
    Applied,
    Restored,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefusalCode {
    UnsupportedOperation,
    ApprovalRequired,
    ApprovalMismatch,
    PrivilegedApplyRequiresTargetLocalHelper,
    PolicyViolation,
    InvalidPlan,
    MissingSurface,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    pub reason_code: RefusalCode,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlResult {
    pub schema_version: String,
    pub result_id: String,
    pub plan_id: String,
    pub target_id: String,
    pub operation_id: String,
    pub risk_tier: RiskTier,
    pub status: ControlResultStatus,
    pub refusal: Option<Refusal>,
    pub restore_lease: Option<RestoreLease>,
    pub restore_attempted: bool,
    pub restore_result: Option<RestoreAttempt>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadPlan {
    pub schema_version: String,
    pub load_id: String,
    pub target_id: String,
    pub load_kind: String,
    pub workers: usize,
    pub duration_seconds: u64,
    pub abort_temp_c: Option<f64>,
    pub created_by: Actor,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LoadResult {
    pub schema_version: String,
    pub result_id: String,
    pub load_id: String,
    pub target_id: String,
    pub status: String,
    pub workers: usize,
    pub duration_ms: u64,
    pub abort_reason: Option<String>,
    pub max_observed_temp_c: Option<f64>,
    pub worker_iterations: Vec<u64>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FactorKind {
    ControlledFactor,
    ObservedCovariate,
    UncontrolledConfounder,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentFactor {
    pub name: String,
    pub kind: FactorKind,
    pub levels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentMatrix {
    pub schema_version: String,
    pub matrix_id: String,
    pub description: String,
    pub factors: Vec<ExperimentFactor>,
    pub warmup_seconds: u64,
    pub cooldown_seconds: u64,
    pub repetitions: u64,
    pub order: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentTrial {
    pub trial_id: String,
    pub factors: BTreeMap<String, String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentRun {
    pub schema_version: String,
    pub run_id: String,
    pub matrix_id: String,
    pub target_id: String,
    pub dry_run: bool,
    pub trials: Vec<ExperimentTrial>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AuditEvent {
    pub schema_version: String,
    pub event_id: String,
    pub run_id: String,
    pub target_id: String,
    pub actor: Actor,
    pub operation: String,
    pub operation_id: Option<String>,
    pub risk_tier: RiskTier,
    pub approval_ref: Option<String>,
    pub restore_lease_ref: Option<String>,
    pub result: String,
    pub policy_version: String,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClaimDecision {
    Supported,
    Blocked,
    Provisional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimTraceEntry {
    pub claim: String,
    pub decision: ClaimDecision,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClaimEvidenceTrace {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub claims: Vec<ClaimTraceEntry>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunArtifactRef {
    pub name: String,
    #[serde(rename = "ref")]
    pub artifact_ref: String,
    pub schema_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunDataQuality {
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub target: String,
    pub mode: String,
    pub started_at_unix_ms: u64,
    pub ended_at_unix_ms: u64,
    pub adc_lab_version: String,
    pub artifacts: Vec<RunArtifactRef>,
    pub audit_ref: String,
    pub claim_trace_ref: Option<String>,
    pub data_quality: RunDataQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolQualification {
    pub schema_version: String,
    pub tool_id: String,
    pub category: ToolCategory,
    pub privilege: PrivilegeLevel,
    pub source: ToolSource,
    pub available: bool,
    pub status: QualificationStatus,
    pub evidence_accepted: bool,
    pub dry_run_required: bool,
    pub evidence_refs: Vec<String>,
    pub checks: Vec<String>,
    pub limitations: Vec<String>,
    pub reason: String,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolQualificationSummary {
    pub schema_version: String,
    pub target_id: String,
    pub qualification_refs: Vec<String>,
    pub evidence_accepted_tool_ids: Vec<String>,
    pub evidence_rejected_tool_ids: Vec<String>,
    pub missing_tool_ids: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointCoverage {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub covered_points: Vec<String>,
    pub blocked_points: Vec<String>,
    pub coverage_status: String,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCostModel {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub capabilities: Vec<String>,
    pub cost_model_status: String,
    pub limitations: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FamiliarizationPack {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub pack_status: String,
    pub artifact_refs: Vec<String>,
    pub supported_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub audit_event_count: usize,
    pub restore_status: String,
    pub claim_trace_ref: Option<String>,
    pub time_unix_ms: u64,
}
