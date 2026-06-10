use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const POLICY_VERSION: &str = "default-lab-policy-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuildInfo {
    pub name: String,
    pub version: String,
    pub git_sha: String,
    pub target_triple: String,
    pub build_profile: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseBinaryManifest {
    pub name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReleaseManifest {
    pub schema_version: String,
    pub version: String,
    pub git_sha: String,
    pub target_triple: String,
    pub binaries: Vec<ReleaseBinaryManifest>,
}

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
pub enum PrivilegeProviderKind {
    SudoHelperOptionA,
    SystemdUnixSocketOptionB,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeProviderAvailability {
    Active,
    Installed,
    PlannedDisabled,
    Missing,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeProviderTransport {
    SudoExec,
    UnixSocket,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeProviderDescriptor {
    pub provider_id: String,
    pub provider_kind: PrivilegeProviderKind,
    pub availability: PrivilegeProviderAvailability,
    pub transport: PrivilegeProviderTransport,
    pub endpoint: String,
    pub root_boundary: String,
    pub operations_allowed: Vec<String>,
    pub approval_required: bool,
    pub audit_required: bool,
    pub restore_required: bool,
    pub default_enabled: bool,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeProviderStatus {
    pub schema_version: String,
    pub target_id: String,
    pub active_provider_id: String,
    pub providers: Vec<PrivilegeProviderDescriptor>,
    pub time_unix_ms: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractEvidenceStatus {
    Measured,
    MeasuredPartial,
    NotControllable,
    UnsafeToRunWithReason,
    NotApplicableWithReason,
    Insufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlatformMechanism {
    pub domain: String,
    pub mechanism_id: String,
    pub description: String,
    pub visibility_status: ContractEvidenceStatus,
    pub platform_control_status: ContractEvidenceStatus,
    pub pressure_probe_status: ContractEvidenceStatus,
    pub evidence_status: ContractEvidenceStatus,
    pub evidence_refs: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlatformMechanismInventory {
    pub schema_version: String,
    pub target_id: String,
    pub target_class: String,
    pub mechanisms: Vec<PlatformMechanism>,
    pub evidence_refs: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoundaryProbe {
    pub probe_id: String,
    pub boundary: String,
    pub controlled_factors: Vec<String>,
    pub observed_covariates: Vec<String>,
    pub uncontrolled_confounders: Vec<String>,
    pub safety_abort_condition: Vec<String>,
    pub restore_cleanup: Vec<String>,
    pub claim_supported: String,
    pub claim_blocked: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BoundaryProbePlan {
    pub schema_version: String,
    pub plan_id: String,
    pub target_id: String,
    pub target_class: String,
    pub probes: Vec<BoundaryProbe>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePressureKind {
    MemoryPressure,
    StorageIo,
    NetworkIo,
    LatencyJitter,
    CpuPressure,
    ThermalPressure,
    ObserverPressure,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceMetric {
    pub metric_id: String,
    pub value: Option<f64>,
    pub unit: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractFactor {
    pub factor_id: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceSideEffect {
    pub resource: String,
    pub status: ContractEvidenceStatus,
    pub summary: String,
    pub metrics: Vec<ResourceMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PressureSafety {
    pub duration_seconds_max: u64,
    pub memory_bytes_max: u64,
    pub storage_bytes_max: u64,
    pub network_bytes_max: u64,
    pub abort_conditions: Vec<String>,
    pub cleanup: Vec<String>,
    pub cleanup_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourcePressureEvidenceClass {
    Smoke,
    PressureInduced,
    PairedPressure,
    BoundaryProbe,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PressureIntensity {
    pub requested: String,
    pub relative_to_target: String,
    pub pressure_effect_observed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PressureEffect {
    pub observed: bool,
    pub basis: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkPressureMode {
    CounterOnly,
    EndpointAttempt,
    BoundedTransfer,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NetworkPressureEvidence {
    pub network_mode: NetworkPressureMode,
    pub endpoint_available: bool,
    pub traffic_generated_bytes: u64,
    pub selection_claim_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PressureCondition {
    pub pressure_kind: String,
    pub governor: Option<String>,
    pub workers: Option<String>,
    pub duration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContractEvidenceGap {
    pub reason: String,
    pub needed_probe: String,
    pub blocking_missing_evidence: Vec<String>,
    pub next_action: String,
    pub owner_surface: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourcePressureResult {
    pub schema_version: String,
    pub result_id: String,
    pub target_id: String,
    pub pressure_kind: ResourcePressureKind,
    pub status: ContractEvidenceStatus,
    pub evidence_class: ResourcePressureEvidenceClass,
    pub intensity: PressureIntensity,
    pub pressure_effect: PressureEffect,
    pub network_evidence: Option<NetworkPressureEvidence>,
    pub condition: PressureCondition,
    pub duration_ms: u64,
    pub controlled_factors: Vec<ContractFactor>,
    pub observed_covariates: Vec<ContractFactor>,
    pub uncontrolled_confounders: Vec<String>,
    pub metrics: Vec<ResourceMetric>,
    pub side_effects: Vec<ResourceSideEffect>,
    pub safety: PressureSafety,
    pub evidence_refs: Vec<String>,
    pub claim_supported: Vec<String>,
    pub claim_blocked: Vec<String>,
    pub next_evidence_needed: Vec<ContractEvidenceGap>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CompositeBoundaryScenario {
    MemoryStorageJitter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompositeBoundaryPhase {
    pub phase_id: String,
    pub pressure_kind: String,
    pub status: ContractEvidenceStatus,
    pub summary: String,
    pub metrics: Vec<ResourceMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompositeBoundaryResult {
    pub schema_version: String,
    pub result_id: String,
    pub target_id: String,
    pub scenario: CompositeBoundaryScenario,
    pub status: ContractEvidenceStatus,
    pub coupling_evidence_class: ResourceCouplingEvidenceClass,
    pub duration_ms: u64,
    pub controlled_factors: Vec<ContractFactor>,
    pub observed_covariates: Vec<ContractFactor>,
    pub uncontrolled_confounders: Vec<String>,
    pub phases: Vec<CompositeBoundaryPhase>,
    pub safety: PressureSafety,
    pub evidence_refs: Vec<String>,
    pub claim_supported: Vec<String>,
    pub claim_blocked: Vec<String>,
    pub next_evidence_needed: Vec<ContractEvidenceGap>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceCouplingEvidenceClass {
    IngredientsOnly,
    CompositeMeasured,
    CouplingNotMeasured,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceCouplingChain {
    pub chain_id: String,
    pub pressure: String,
    pub platform_response: String,
    pub secondary_pressure: String,
    pub performance_degradation: String,
    pub recovery_behavior: String,
    pub status: ContractEvidenceStatus,
    pub coupling_evidence_class: ResourceCouplingEvidenceClass,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<ContractEvidenceGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResourceCouplingReport {
    pub schema_version: String,
    pub report_id: String,
    pub target_id: String,
    pub report_status: ContractEvidenceStatus,
    pub chains: Vec<ResourceCouplingChain>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetOperatingContractStatus {
    MeasuredPartial,
    MeasuredReference,
    Insufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatingRuleCategory {
    AllowedDefault,
    BurstOnly,
    DegradedModeTrigger,
    ForbiddenPattern,
    BlockedClaim,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContractConfidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatingRuleSource {
    MeasuredTargetEvidence,
    GenericLabRule,
    EvidenceNeededRule,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingContractRule {
    pub rule_id: String,
    pub category: OperatingRuleCategory,
    pub statement: String,
    pub rule_source: OperatingRuleSource,
    pub derivation: String,
    pub evidence_refs: Vec<String>,
    pub confidence: ContractConfidence,
    pub allowed_design: Vec<String>,
    pub blocked_design: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingBoundary {
    pub boundary_id: String,
    pub statement: String,
    pub status: ContractEvidenceStatus,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<ContractEvidenceGap>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetOperatingContract {
    pub schema_version: String,
    pub target_id: String,
    pub target_class: String,
    pub contract_status: TargetOperatingContractStatus,
    pub rules: Vec<OperatingContractRule>,
    pub boundaries: Vec<OperatingBoundary>,
    pub unknowns: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum OperatingContractPackStatus {
    ObservationalReadOnly,
    ReadOnlyPlusPressureProbes,
    ExploratoryPressureSmoke,
    ControlledGovernorSubset,
    ControlledOperatingPointSubset,
    CompositeCouplingProbe,
    PlatformOperatingContractCandidate,
    PlatformOperatingContractReference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunSetRole {
    Primary,
    IncludedEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunSetEntry {
    pub run_id: String,
    pub run_dir: String,
    pub role: RunSetRole,
    pub evidence_refs: Vec<String>,
    pub operations_summary: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RunSetManifest {
    pub schema_version: String,
    pub run_set_id: String,
    pub target_id: String,
    pub target_class: String,
    pub pack_status: OperatingContractPackStatus,
    pub runs: Vec<RunSetEntry>,
    pub operations_summary: BTreeMap<String, String>,
    pub included_surfaces: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub blocked_evidence: Vec<ContractEvidenceGap>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MultiRunOperatingContract {
    pub schema_version: String,
    pub contract_id: String,
    pub target_id: String,
    pub target_class: String,
    pub pack_status: OperatingContractPackStatus,
    pub contract_status: TargetOperatingContractStatus,
    pub run_set_ref: Option<String>,
    pub source_runs: Vec<RunSetEntry>,
    pub rules: Vec<OperatingContractRule>,
    pub boundaries: Vec<OperatingBoundary>,
    pub unknowns: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeDoctorStatus {
    Ready,
    Degraded,
    OperatorSetupRequired,
    UnsupportedTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeDoctorCheckStatus {
    Pass,
    Fail,
    Warning,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeDoctorCheck {
    pub check_id: String,
    pub status: PrivilegeDoctorCheckStatus,
    pub summary: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeDoctorReport {
    pub schema_version: String,
    pub target_id: String,
    pub helper_path: String,
    pub helper_installed: bool,
    pub root_owned: Option<bool>,
    pub world_writable: Option<bool>,
    pub sudo_non_interactive_available: bool,
    pub helper_version: Option<String>,
    pub status: PrivilegeDoctorStatus,
    pub checks: Vec<PrivilegeDoctorCheck>,
    pub next_action: String,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeSetupPlanKind {
    Install,
    Uninstall,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PrivilegeSetupPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub target_id: String,
    pub plan_kind: PrivilegeSetupPlanKind,
    pub helper_path: String,
    pub operator_steps: Vec<String>,
    pub commands: Vec<String>,
    pub verification_commands: Vec<String>,
    pub warnings: Vec<String>,
    pub time_unix_ms: u64,
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
pub struct LoadSafetyMonitorPlan {
    pub sample_interval_ms: u64,
    pub thermal_abort_c: Option<f64>,
    pub operator_abort_enabled: bool,
    pub restore_on_abort: LoadRestoreOnAbortPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadRestoreOnAbortPolicy {
    NotRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoadRestoreOnAbortStatus {
    NotRequired,
    NotConfigured,
    Attempted,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LoadSafetyMonitorResult {
    pub sample_interval_ms: u64,
    pub samples: u64,
    pub thermal_surface_available: bool,
    pub operator_abort_observed: bool,
    pub restore_on_abort_status: LoadRestoreOnAbortStatus,
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
    pub safety_monitor: LoadSafetyMonitorPlan,
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
    pub safety_monitor: LoadSafetyMonitorResult,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClass {
    IdleObserve,
    SyntheticCpu,
    ApplicationWorkload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClaimBoundary {
    ExploratoryOnly,
    SyntheticShortSmokeOnly,
    NotSelectionEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkloadRequirements {
    pub thermal_celsius_max: Option<f64>,
    pub max_abort_count: Option<u64>,
    pub memory_mb_max: Option<u64>,
    pub latency_p95_ms_max: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkloadProfile {
    pub schema_version: String,
    pub workload_id: String,
    pub description: String,
    pub workload_class: WorkloadClass,
    pub duration_seconds: u64,
    pub requirements: WorkloadRequirements,
    pub measurement_requirements: Vec<String>,
    pub claim_boundary: WorkloadClaimBoundary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetCapabilityStatus {
    NoEvidence,
    ExploratoryPartial,
    MeasuredShortSmoke,
    InsufficientForSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservedCapabilityResults {
    pub max_temp_c: Option<f64>,
    pub abort_count: u64,
    pub cpu_frequency_range_khz: Option<Vec<u64>>,
    pub memory_available_kb_min: Option<u64>,
    pub max_load_duration_ms: u64,
    pub max_observation_duration_ms: u64,
    pub load_result_count: u64,
    pub observation_sample_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct TargetCapabilityProfile {
    pub schema_version: String,
    pub target_id: String,
    pub workload_id: String,
    pub evidence_pack_ref: Option<String>,
    pub capability_status: TargetCapabilityStatus,
    pub selection_ready: bool,
    pub observed_results: ObservedCapabilityResults,
    pub supported_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub evidence_refs: Vec<String>,
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
    pub artifact_refs: Vec<String>,
    pub failure: Option<String>,
    pub started_at_unix_ms: Option<u64>,
    pub ended_at_unix_ms: Option<u64>,
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
    pub inconsistent: Vec<String>,
    pub notes: Vec<String>,
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
    pub adc_lab_git_sha: String,
    pub adc_lab_target_version: String,
    pub adc_lab_target_git_sha: String,
    pub release_tag: String,
    pub release_asset: String,
    pub release_asset_sha256: String,
    pub binary_sha256: BTreeMap<String, String>,
    pub operations_summary: BTreeMap<String, String>,
    pub operation_audit_refs: BTreeMap<String, String>,
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
    pub qualification_scope: String,
    pub tool_version: Option<String>,
    pub tool_sha256: Option<String>,
    pub output_schema_ref: Option<String>,
    pub dry_run_ref: Option<String>,
    pub manual_comparison_ref: Option<String>,
    pub static_safety_review_ref: Option<String>,
    pub validated_output_bytes: Option<u64>,
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
    pub tools: Vec<ToolQualificationSummaryEntry>,
    pub qualification_refs: Vec<String>,
    pub evidence_accepted_tool_ids: Vec<String>,
    pub evidence_rejected_tool_ids: Vec<String>,
    pub missing_tool_ids: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolQualificationSummaryEntry {
    pub tool_id: String,
    pub status: String,
    pub evidence_accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointCoverage {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub coverage_status: OperatingPointCoverageStatus,
    pub observed_points: Vec<OperatingPointCoveragePoint>,
    pub controlled_points: Vec<OperatingPointCoveragePoint>,
    pub blocked_points: Vec<OperatingPointBlockedPoint>,
    pub claim_boundaries: Vec<OperatingPointClaimBoundary>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatingPointCoverageStatus {
    ObservationalOnly,
    ControlledSubset,
    ControlledFull,
    NotControllable,
    BlockedUnsafe,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatingPointEvidenceClass {
    PassiveObservation,
    BoundedLoad,
    PrivilegedControl,
    MissingEvidence,
    SafetyBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointCoveragePoint {
    pub factor_id: String,
    pub level: String,
    pub coverage_status: OperatingPointCoverageStatus,
    pub evidence_class: OperatingPointEvidenceClass,
    pub evidence_refs: Vec<String>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointBlockedPoint {
    pub factor_id: String,
    pub requested_level: Option<String>,
    pub coverage_status: OperatingPointCoverageStatus,
    pub reason: String,
    pub next_evidence_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatingPointClaimBoundary {
    pub claim: String,
    pub decision: ClaimDecision,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCostModel {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub model_status: CapabilityCostModelStatus,
    pub capabilities: Vec<CapabilityEvidence>,
    pub architecture_options: Vec<ArchitectureOptionEvidence>,
    pub blocked_claims: Vec<CapabilityClaimBoundary>,
    pub limitations: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub time_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCostModelStatus {
    InsufficientEvidence,
    HostFallbackOnly,
    TargetEvidencePartial,
    CostModelReadyForLabClaims,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityClass {
    Cpu,
    Memory,
    Thermal,
    FrequencyControl,
    Load,
    Gpu,
    Npu,
    Dsp,
    Storage,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityEvidenceStatus {
    Observed,
    MissingEvidence,
    NeedsProbe,
    Provisional,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CostDimensionKind {
    Cpu,
    Memory,
    Thermal,
    Wakeups,
    StorageWrites,
    FlashWear,
    Battery,
    LatencyJitter,
    Network,
    ObserverOverhead,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CostEvidenceStatus {
    MeasuredPartial,
    MissingEvidence,
    NotMeasured,
    Provisional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityEvidence {
    pub capability_id: String,
    pub capability_class: CapabilityClass,
    pub status: CapabilityEvidenceStatus,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub cost_dimensions: Vec<CapabilityCostDimension>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityCostDimension {
    pub dimension: CostDimensionKind,
    pub status: CostEvidenceStatus,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub limitation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchitectureOptionEvidence {
    pub option_id: String,
    pub summary: String,
    pub decision: ClaimDecision,
    pub rationale: String,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapabilityClaimBoundary {
    pub claim: String,
    pub decision: ClaimDecision,
    pub evidence_refs: Vec<String>,
    pub next_evidence_needed: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FamiliarizationPack {
    pub schema_version: String,
    pub run_id: String,
    pub target_id: String,
    pub pack_status: String,
    pub operations_summary: BTreeMap<String, String>,
    pub artifact_refs: Vec<String>,
    pub supported_claims: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub next_evidence_needed: Vec<String>,
    pub audit_event_count: usize,
    pub restore_status: String,
    pub claim_trace_ref: Option<String>,
    pub tool_qualification_summary_ref: Option<String>,
    pub time_unix_ms: u64,
}
