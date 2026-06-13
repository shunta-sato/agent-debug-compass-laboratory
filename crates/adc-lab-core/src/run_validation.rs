use crate::contracts::{
    Actor, ApprovalRecord, BuildInfo, ControlPlan, ControlResult, ControlResultStatus, HealthCheck,
    PrivilegeDoctorReport, RefusalCode, ReleaseManifest,
};
use crate::control::canonical_plan_digest;
use crate::evidence::{Artifact, DataQuality, DataQualityLevel, Kind, Status};
use crate::fsutil::read_json;
use crate::ids::{new_id, now_unix_ms};
use crate::probe::LoadPayload;
use crate::{LabError, LabResult, RunContext};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const FULLSET_PROFILE: &str = "target-operating-contract-fullset";
pub const LEGACY_RUN_VALIDATION_MISSING_RUN_SET_ID: &str =
    "legacy_run_validation_missing_run_set_identity";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernorValidity {
    Measured,
    MeasuredPartial,
    Insufficient,
    Refused,
    Contaminated,
    NotApplicable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GovernorValidation {
    pub governor: String,
    pub validity: GovernorValidity,
    pub plan_ref: Option<String>,
    pub approval_ref: Option<String>,
    pub control_result_ref: Option<String>,
    pub load_ref: Option<String>,
    pub restore_result_ref: Option<String>,
    pub health_check_ref: Option<String>,
    pub messages: Vec<String>,
    pub next_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunValidationGap {
    pub code: String,
    pub governor: Option<String>,
    pub message: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunValidationPayload {
    pub profile: String,
    pub requested_governors: Vec<String>,
    #[serde(default)]
    pub workflow_recommendation_ref: Option<String>,
    #[serde(default)]
    pub collect_plan_ref: Option<String>,
    #[serde(default)]
    pub collect_plan_digest: Option<String>,
    #[serde(default = "legacy_run_set_id")]
    pub subject_run_set_id: String,
    #[serde(default)]
    pub included_run_refs: Vec<String>,
    #[serde(default = "default_validation_profile")]
    pub validation_profile: String,
    #[serde(default)]
    pub expected_governors: Vec<String>,
    #[serde(default = "unknown_target_id")]
    pub target_id: String,
    #[serde(default = "unknown_target_class")]
    pub target_class: String,
    #[serde(default)]
    pub version_set: RunValidationVersionSet,
    #[serde(default)]
    pub version_skew_policy: VersionSkewPolicyResult,
    #[serde(default)]
    pub version_skew_override: bool,
    pub governor_results: Vec<GovernorValidation>,
    pub overall_validity: GovernorValidity,
    pub gaps: Vec<RunValidationGap>,
    pub audit_refs: Vec<String>,
}
impl RunValidationPayload {
    pub fn has_run_set_identity(&self) -> bool {
        self.subject_run_set_id != LEGACY_RUN_VALIDATION_MISSING_RUN_SET_ID
            && !self.subject_run_set_id.trim().is_empty()
            && !self.included_run_refs.is_empty()
    }
}
#[derive(Debug, Clone)]
pub struct RunValidationInput {
    pub subject_run: RunContext,
    pub include_runs: Vec<RunContext>,
    pub requested_governors: Vec<String>,
    pub workflow_recommendation_ref: Option<String>,
    pub collect_plan_ref: Option<String>,
    pub collect_plan_digest: Option<String>,
    pub target_id: Option<String>,
    pub target_class: Option<String>,
    pub allow_version_skew: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ToolVersionRole {
    ControllerAdcLab,
    TargetLocalAdcLab,
    TargetRunner,
    PrivilegedHelper,
    ReleaseManifest,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolVersionRecord {
    pub role: ToolVersionRole,
    pub tool_name: String,
    pub version: String,
    pub git_sha: String,
    pub target_triple: String,
    pub build_profile: String,
    pub artifact_ref: String,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunValidationVersionSet {
    pub records: Vec<ToolVersionRecord>,
    pub skew_detected: bool,
    pub skew_reasons: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VersionSkewPolicyResult {
    #[default]
    NoSkewDetected,
    BlockedByVersionSkew,
    OverrideRecordedStillBlocked,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GovernorSweepPolicyPayload {
    pub target_id: String,
    pub governors: Vec<String>,
    pub duration_seconds_max: u64,
    pub thermal_celsius_abort: Option<f64>,
    pub expires_at_unix_ms: u64,
    pub requested_by: Actor,
    pub approved_by: Option<Actor>,
    pub policy_state: GovernorSweepPolicyState,
    pub policy_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GovernorSweepPolicyState {
    Requested,
    Approved,
}

pub fn governor_sweep_policy_digest(payload: &GovernorSweepPolicyPayload) -> LabResult<String> {
    let mut canonical = payload.clone();
    canonical.policy_digest.clear();
    let bytes = serde_json::to_vec(&canonical)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

#[derive(Debug, Clone)]
struct PlanEntry {
    artifact_ref: String,
    value: ControlPlan,
}

#[derive(Debug, Clone)]
struct ApprovalEntry {
    artifact_ref: String,
    value: ApprovalRecord,
}

#[derive(Debug, Clone)]
struct ControlResultEntry {
    artifact_ref: String,
    value: ControlResult,
}

#[derive(Debug, Clone)]
struct LoadEntry {
    artifact_ref: String,
    value: Artifact<LoadPayload>,
}

#[derive(Debug, Clone, Default)]
struct ValidationIndex {
    plans: Vec<PlanEntry>,
    approvals: Vec<ApprovalEntry>,
    results: Vec<ControlResultEntry>,
    loads: Vec<LoadEntry>,
    health_check_ref: Option<String>,
    health_check_ok: bool,
    audit_refs: Vec<String>,
}

pub fn validate_fullset_run(
    run: &RunContext,
    requested_governors: Vec<String>,
) -> LabResult<Artifact<RunValidationPayload>> {
    validate_fullset_run_set(RunValidationInput {
        subject_run: run.clone(),
        include_runs: Vec::new(),
        requested_governors,
        workflow_recommendation_ref: None,
        collect_plan_ref: None,
        collect_plan_digest: None,
        target_id: None,
        target_class: None,
        allow_version_skew: false,
    })
}

pub fn validate_fullset_run_set(
    input: RunValidationInput,
) -> LabResult<Artifact<RunValidationPayload>> {
    let mut runs = vec![input.subject_run.clone()];
    runs.extend(input.include_runs.clone());
    let index = ValidationIndex::load_runs(&runs)?;
    let requested_governors = normalize_governors(input.requested_governors);
    let mut governor_results = Vec::new();
    let mut gaps = Vec::new();

    for governor in &requested_governors {
        let result = validate_governor(&index, governor)?;
        gaps.extend(gaps_for_result(&result));
        governor_results.push(result);
    }

    let version_set = version_set_for_runs(&runs)?;
    let version_skew_policy = version_skew_policy(&version_set, input.allow_version_skew);
    apply_version_skew_policy(
        &mut governor_results,
        &mut gaps,
        &version_skew_policy,
        &version_set,
    );
    let overall_validity = overall_validity(&governor_results);
    let target_id = input
        .target_id
        .filter(|value| value != "unknown-target")
        .unwrap_or_else(|| target_id_from_index(&index));
    let target_class = input
        .target_class
        .unwrap_or_else(|| "unknown-target-class".to_string());
    let subject_run_set_id = run_set_id(&runs);
    let included_run_refs = run_refs(&runs)?;
    let audit_refs = index.audit_refs.clone();
    let mut artifact = Artifact::new(
        Kind::ReportRunValidation,
        new_id("RUN-VALIDATION"),
        input.subject_run.run_id.clone(),
        target_id.clone(),
        envelope_status(&overall_validity),
        RunValidationPayload {
            profile: FULLSET_PROFILE.to_string(),
            requested_governors: requested_governors.clone(),
            workflow_recommendation_ref: input.workflow_recommendation_ref,
            collect_plan_ref: input.collect_plan_ref,
            collect_plan_digest: input.collect_plan_digest,
            subject_run_set_id,
            included_run_refs,
            validation_profile: FULLSET_PROFILE.to_string(),
            expected_governors: requested_governors,
            target_id,
            target_class,
            version_set,
            version_skew_policy,
            version_skew_override: input.allow_version_skew,
            governor_results,
            overall_validity,
            gaps,
            audit_refs,
        },
        now_unix_ms(),
    );
    artifact.evidence_refs = artifact
        .payload
        .governor_results
        .iter()
        .flat_map(governor_evidence_refs)
        .collect();
    artifact.evidence_refs.sort();
    artifact.evidence_refs.dedup();
    artifact.data_quality = DataQuality {
        level: if matches!(artifact.status, Status::Measured) {
            DataQualityLevel::Complete
        } else {
            DataQualityLevel::Degraded
        },
        notes: vec!["target operating contract full-set validation".to_string()],
    };
    Ok(artifact)
}

pub fn render_run_validation_gaps(validation: &Artifact<RunValidationPayload>) -> String {
    let mut out = String::new();
    out.push_str("# Run Validation Gaps\n\n");
    out.push_str(&format!("- validation_id: `{}`\n", validation.id));
    out.push_str(&format!("- profile: `{}`\n", validation.payload.profile));
    out.push_str(&format!(
        "- overall_validity: `{:?}`\n\n",
        validation.payload.overall_validity
    ));
    if validation.payload.gaps.is_empty() {
        out.push_str("No validation gaps were found.\n");
        return out;
    }
    for gap in &validation.payload.gaps {
        out.push_str(&format!(
            "- `{}`{}: {}\n",
            gap.code,
            gap.governor
                .as_ref()
                .map(|governor| format!(" for `{governor}`"))
                .unwrap_or_default(),
            gap.message
        ));
    }
    out
}

fn validate_governor(index: &ValidationIndex, governor: &str) -> LabResult<GovernorValidation> {
    let Some(plan) = index
        .plans
        .iter()
        .find(|plan| plan.value.operation.desired_state.governor == governor)
    else {
        return Ok(governor_result(
            governor,
            GovernorValidity::Unknown,
            vec!["no control plan for requested governor".to_string()],
        ));
    };

    let approval = matching_approval(index, &plan.value)?;
    let control_result = primary_control_result(index, &plan.value.plan_id);

    let approval_ref = approval.map(|approval| approval.artifact_ref.clone());
    let control_result_ref = control_result.map(|result| result.artifact_ref.clone());

    if let Some(result) = control_result {
        if result.value.status == ControlResultStatus::Refused {
            return Ok(GovernorValidation {
                governor: governor.to_string(),
                validity: GovernorValidity::Refused,
                plan_ref: Some(plan.artifact_ref.clone()),
                approval_ref,
                control_result_ref,
                load_ref: None,
                restore_result_ref: None,
                health_check_ref: index.health_check_ref.clone(),
                messages: vec![refusal_message(&result.value)],
                next_evidence: vec!["collect matching approval and rerun control apply".to_string()],
            });
        }
        if result.value.status == ControlResultStatus::Failed {
            return Ok(GovernorValidation {
                governor: governor.to_string(),
                validity: GovernorValidity::Insufficient,
                plan_ref: Some(plan.artifact_ref.clone()),
                approval_ref,
                control_result_ref,
                load_ref: None,
                restore_result_ref: None,
                health_check_ref: index.health_check_ref.clone(),
                messages: vec!["control apply failed".to_string()],
                next_evidence: vec!["rerun apply and restore with verified result".to_string()],
            });
        }
    }

    let Some(applied) =
        control_result.filter(|result| result.value.status == ControlResultStatus::Applied)
    else {
        return Ok(GovernorValidation {
            governor: governor.to_string(),
            validity: GovernorValidity::Insufficient,
            plan_ref: Some(plan.artifact_ref.clone()),
            approval_ref,
            control_result_ref,
            load_ref: None,
            restore_result_ref: None,
            health_check_ref: index.health_check_ref.clone(),
            messages: vec!["no applied control result for requested governor".to_string()],
            next_evidence: vec!["apply the matching governor plan".to_string()],
        });
    };

    let linked_load = linked_load(index, &applied.artifact_ref, governor);
    let any_unlinked_load = index
        .loads
        .iter()
        .find(|load| load.value.kind == Kind::Load);
    let restore_result = index.results.iter().find(|result| {
        result.value.plan_id == plan.value.plan_id
            && result.value.status == ControlResultStatus::Restored
    });

    let health_ref = index.health_check_ref.clone();
    let restore_ref = restore_result.map(|result| result.artifact_ref.clone());

    let Some(load) = linked_load else {
        return Ok(GovernorValidation {
            governor: governor.to_string(),
            validity: if any_unlinked_load.is_some() {
                GovernorValidity::Contaminated
            } else {
                GovernorValidity::Insufficient
            },
            plan_ref: Some(plan.artifact_ref.clone()),
            approval_ref,
            control_result_ref: Some(applied.artifact_ref.clone()),
            load_ref: any_unlinked_load.map(|load| load.artifact_ref.clone()),
            restore_result_ref: restore_ref,
            health_check_ref: health_ref,
            messages: vec![
                "bounded load evidence is missing an explicit control-result or operating-point link"
                    .to_string(),
            ],
            next_evidence: vec![
                "rerun load through governor sweep so load evidence carries a control link".to_string(),
            ],
        });
    };

    if restore_ref.is_none() || health_ref.is_none() || !index.health_check_ok {
        return Ok(GovernorValidation {
            governor: governor.to_string(),
            validity: GovernorValidity::MeasuredPartial,
            plan_ref: Some(plan.artifact_ref.clone()),
            approval_ref,
            control_result_ref: Some(applied.artifact_ref.clone()),
            load_ref: Some(load.artifact_ref.clone()),
            restore_result_ref: restore_ref,
            health_check_ref: health_ref,
            messages: vec![
                "load is linked, but restore or healthy post-restore evidence is incomplete"
                    .to_string(),
            ],
            next_evidence: vec!["restore and run post-restore health check".to_string()],
        });
    }

    Ok(GovernorValidation {
        governor: governor.to_string(),
        validity: GovernorValidity::Measured,
        plan_ref: Some(plan.artifact_ref.clone()),
        approval_ref,
        control_result_ref: Some(applied.artifact_ref.clone()),
        load_ref: Some(load.artifact_ref.clone()),
        restore_result_ref: restore_ref,
        health_check_ref: health_ref,
        messages: vec!["governor evidence is fully linked and restored".to_string()],
        next_evidence: Vec::new(),
    })
}

fn matching_approval<'a>(
    index: &'a ValidationIndex,
    plan: &ControlPlan,
) -> LabResult<Option<&'a ApprovalEntry>> {
    let digest = canonical_plan_digest(plan)?;
    Ok(index.approvals.iter().find(|approval| {
        approval.value.approved_plan_id == plan.plan_id
            && approval.value.approved_plan_digest == digest
            && approval.value.approved_operation == plan.operation
    }))
}

fn primary_control_result<'a>(
    index: &'a ValidationIndex,
    plan_id: &str,
) -> Option<&'a ControlResultEntry> {
    [
        ControlResultStatus::Refused,
        ControlResultStatus::Failed,
        ControlResultStatus::Applied,
        ControlResultStatus::DryRunOk,
        ControlResultStatus::Restored,
    ]
    .into_iter()
    .find_map(|status| {
        index
            .results
            .iter()
            .find(|result| result.value.plan_id == plan_id && result.value.status == status)
    })
}

fn linked_load<'a>(
    index: &'a ValidationIndex,
    control_result_ref: &str,
    governor: &str,
) -> Option<&'a LoadEntry> {
    index.loads.iter().find(|load| {
        let snapshot_governor = load
            .value
            .payload
            .operating_point_snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.governor.as_deref());
        match load.value.payload.control_result_ref.as_deref() {
            Some(recorded_ref) => {
                recorded_ref == control_result_ref
                    && snapshot_governor
                        .is_none_or(|recorded_governor| recorded_governor == governor)
            }
            None => false,
        }
    })
}

fn governor_result(
    governor: &str,
    validity: GovernorValidity,
    messages: Vec<String>,
) -> GovernorValidation {
    GovernorValidation {
        governor: governor.to_string(),
        validity,
        plan_ref: None,
        approval_ref: None,
        control_result_ref: None,
        load_ref: None,
        restore_result_ref: None,
        health_check_ref: None,
        messages,
        next_evidence: vec!["collect typed governor control evidence".to_string()],
    }
}

fn refusal_message(result: &ControlResult) -> String {
    let Some(refusal) = &result.refusal else {
        return "control apply was refused".to_string();
    };
    match refusal.reason_code {
        RefusalCode::ApprovalMismatch => {
            format!(
                "control apply refused by approval mismatch: {}",
                refusal.message
            )
        }
        _ => format!("control apply refused: {}", refusal.message),
    }
}

fn gaps_for_result(result: &GovernorValidation) -> Vec<RunValidationGap> {
    if result.validity == GovernorValidity::Measured {
        return Vec::new();
    }
    vec![RunValidationGap {
        code: format!("{:?}", result.validity).to_ascii_lowercase(),
        governor: Some(result.governor.clone()),
        message: result.messages.join("; "),
        evidence_refs: governor_evidence_refs(result),
    }]
}

fn governor_evidence_refs(result: &GovernorValidation) -> Vec<String> {
    [
        result.plan_ref.clone(),
        result.approval_ref.clone(),
        result.control_result_ref.clone(),
        result.load_ref.clone(),
        result.restore_result_ref.clone(),
        result.health_check_ref.clone(),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn overall_validity(results: &[GovernorValidation]) -> GovernorValidity {
    if results.is_empty() {
        return GovernorValidity::Unknown;
    }
    if results
        .iter()
        .all(|result| result.validity == GovernorValidity::Measured)
    {
        GovernorValidity::Measured
    } else if results.iter().any(|result| {
        matches!(
            result.validity,
            GovernorValidity::Contaminated | GovernorValidity::Refused
        )
    }) {
        GovernorValidity::Contaminated
    } else if results
        .iter()
        .any(|result| result.validity == GovernorValidity::MeasuredPartial)
    {
        GovernorValidity::MeasuredPartial
    } else if results
        .iter()
        .any(|result| result.validity == GovernorValidity::Insufficient)
    {
        GovernorValidity::Insufficient
    } else {
        GovernorValidity::Unknown
    }
}

fn version_skew_policy(
    version_set: &RunValidationVersionSet,
    allow_version_skew: bool,
) -> VersionSkewPolicyResult {
    if !version_set.skew_detected {
        VersionSkewPolicyResult::NoSkewDetected
    } else if allow_version_skew {
        VersionSkewPolicyResult::OverrideRecordedStillBlocked
    } else {
        VersionSkewPolicyResult::BlockedByVersionSkew
    }
}

fn apply_version_skew_policy(
    results: &mut [GovernorValidation],
    gaps: &mut Vec<RunValidationGap>,
    policy: &VersionSkewPolicyResult,
    version_set: &RunValidationVersionSet,
) {
    if matches!(policy, VersionSkewPolicyResult::NoSkewDetected) {
        return;
    }

    let code = match policy {
        VersionSkewPolicyResult::BlockedByVersionSkew => "blocked_by_version_skew",
        VersionSkewPolicyResult::OverrideRecordedStillBlocked => {
            "version_skew_override_still_blocked"
        }
        VersionSkewPolicyResult::NoSkewDetected => unreachable!(),
    };
    let message = match policy {
        VersionSkewPolicyResult::BlockedByVersionSkew => {
            "version skew blocks full-set measured claims by default"
        }
        VersionSkewPolicyResult::OverrideRecordedStillBlocked => {
            "version skew override was recorded, but full-set measured claims remain blocked"
        }
        VersionSkewPolicyResult::NoSkewDetected => unreachable!(),
    };
    let version_refs = version_set
        .records
        .iter()
        .map(|record| record.artifact_ref.clone())
        .collect::<Vec<_>>();

    for result in results {
        if result.validity != GovernorValidity::Contaminated
            && result.validity != GovernorValidity::Refused
            && result.validity != GovernorValidity::NotApplicable
        {
            result.validity = GovernorValidity::Insufficient;
        }
        result.messages.push(message.to_string());
        result.next_evidence.push(
            "rerun controller and target-local workflow with matching adc-lab versions".to_string(),
        );
        gaps.push(RunValidationGap {
            code: code.to_string(),
            governor: Some(result.governor.clone()),
            message: format!("{}: {}", message, version_set.skew_reasons.join("; ")),
            evidence_refs: version_refs.clone(),
        });
    }
}

fn envelope_status(validity: &GovernorValidity) -> Status {
    match validity {
        GovernorValidity::Measured => Status::Measured,
        GovernorValidity::MeasuredPartial => Status::MeasuredPartial,
        GovernorValidity::NotApplicable => Status::NotApplicable {
            reason: "full-set validation is not applicable".to_string(),
        },
        GovernorValidity::Refused => Status::Refused {
            code: crate::evidence::EvidenceRefusalCode::PolicyViolation,
            message: "full-set validation includes refused control evidence".to_string(),
        },
        GovernorValidity::Contaminated => Status::UnsafeBlocked {
            reason: "full-set validation contains contaminated evidence".to_string(),
        },
        GovernorValidity::Insufficient | GovernorValidity::Unknown => Status::Insufficient,
    }
}

fn normalize_governors(governors: Vec<String>) -> Vec<String> {
    let mut governors = governors
        .into_iter()
        .flat_map(|item| {
            item.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    governors.sort();
    governors.dedup();
    governors
}

fn legacy_run_set_id() -> String {
    LEGACY_RUN_VALIDATION_MISSING_RUN_SET_ID.to_string()
}

fn default_validation_profile() -> String {
    FULLSET_PROFILE.to_string()
}

fn unknown_target_id() -> String {
    "unknown-target".to_string()
}

fn unknown_target_class() -> String {
    "unknown-target-class".to_string()
}

impl ValidationIndex {
    fn load_runs(runs: &[RunContext]) -> LabResult<Self> {
        let mut index = Self::default();
        for run in runs {
            index.plans.extend(read_plan_entries(run)?);
            index.approvals.extend(read_approval_entries(run)?);
            index.results.extend(read_result_entries(run)?);
            index.loads.extend(read_load_entries(run)?);
            index.load_health_and_audit(run)?;
        }
        Ok(index)
    }

    fn load_health_and_audit(&mut self, run: &RunContext) -> LabResult<()> {
        let health_path = run.run_dir.join("health/restore_health_check.json");
        if health_path.exists() {
            let health_ref = run.artifact_uri(&health_path)?;
            let health: HealthCheck = read_json(&health_path)?;
            if health.status == "ok" || self.health_check_ref.is_none() {
                self.health_check_ref = Some(health_ref);
                self.health_check_ok = health.status == "ok";
            }
        }
        let audit_path = run.run_dir.join("audit.jsonl");
        if audit_path.exists() {
            self.audit_refs.push(run.artifact_uri(&audit_path)?);
        }
        Ok(())
    }
}

fn read_plan_entries(run: &RunContext) -> LabResult<Vec<PlanEntry>> {
    let mut entries = Vec::new();
    for path in json_files(run.run_dir.join("plans"))? {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".result.json") {
            continue;
        }
        entries.push(PlanEntry {
            artifact_ref: run.artifact_uri(&path)?,
            value: read_json(&path)?,
        });
    }
    Ok(entries)
}

fn read_approval_entries(run: &RunContext) -> LabResult<Vec<ApprovalEntry>> {
    let mut entries = Vec::new();
    for path in json_files(run.run_dir.join("approvals"))? {
        let value: serde_json::Value = read_json(&path)?;
        if value.get("schema_version").and_then(|value| value.as_str())
            != Some("lab.approval_record.v1")
        {
            continue;
        }
        entries.push(ApprovalEntry {
            artifact_ref: run.artifact_uri(&path)?,
            value: serde_json::from_value(value)?,
        });
    }
    Ok(entries)
}

fn read_result_entries(run: &RunContext) -> LabResult<Vec<ControlResultEntry>> {
    let mut entries = Vec::new();
    for path in json_files(run.run_dir.join("plans"))? {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.ends_with(".result.json") {
            continue;
        }
        entries.push(ControlResultEntry {
            artifact_ref: run.artifact_uri(&path)?,
            value: read_json(&path)?,
        });
    }
    Ok(entries)
}

fn read_load_entries(run: &RunContext) -> LabResult<Vec<LoadEntry>> {
    let mut entries = Vec::new();
    for path in recursive_json_files(run.run_dir.join("load"))? {
        let artifact: Artifact<LoadPayload> = read_json(&path)?;
        if artifact.kind == Kind::Load {
            entries.push(LoadEntry {
                artifact_ref: run.artifact_uri(&path)?,
                value: artifact,
            });
        }
    }
    Ok(entries)
}

fn json_files(dir: PathBuf) -> LabResult<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|source| LabError::IoWithPath {
        path: dir.clone(),
        source,
    })? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn recursive_json_files(dir: PathBuf) -> LabResult<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_json_files(&dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_json_files(path: &Path, files: &mut Vec<PathBuf>) -> LabResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|source| LabError::IoWithPath {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(LabError::Validation(format!(
            "validation refuses symlink artifact path: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path).map_err(|source| LabError::IoWithPath {
        path: path.to_path_buf(),
        source,
    })? {
        collect_json_files(&entry?.path(), files)?;
    }
    Ok(())
}

fn target_id_from_index(index: &ValidationIndex) -> String {
    index
        .plans
        .first()
        .map(|plan| plan.value.target_id.clone())
        .or_else(|| {
            index
                .results
                .first()
                .map(|result| result.value.target_id.clone())
        })
        .or_else(|| index.loads.first().map(|load| load.value.target_id.clone()))
        .unwrap_or_else(|| "unknown-target".to_string())
}

pub fn digest_file_sha256(path: &Path) -> LabResult<String> {
    let bytes = fs::read(path).map_err(|source| LabError::IoWithPath {
        path: path.to_path_buf(),
        source,
    })?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn run_set_id(runs: &[RunContext]) -> String {
    let mut values = runs
        .iter()
        .map(|run| format!("{}:{}", run.run_id, run.run_dir.display()))
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    let digest = Sha256::digest(values.join("\n").as_bytes());
    format!("RUN-SET-{digest:x}").chars().take(24).collect()
}

fn run_refs(runs: &[RunContext]) -> LabResult<Vec<String>> {
    runs.iter()
        .map(|run| {
            let context_path = run.run_dir.join("run_context.json");
            if context_path.exists() {
                run.artifact_uri(context_path)
            } else {
                Ok(format!("run-dir:{}", run.run_dir.display()))
            }
        })
        .collect()
}

fn version_set_for_runs(runs: &[RunContext]) -> LabResult<RunValidationVersionSet> {
    let mut records = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        let controller_path = run.run_dir.join("tools/adc-lab.version.json");
        if controller_path.exists() {
            let role = if index == 0 {
                ToolVersionRole::ControllerAdcLab
            } else {
                ToolVersionRole::TargetLocalAdcLab
            };
            records.push(build_info_record(run, &controller_path, role)?);
        }
        let target_path = run.run_dir.join("tools/adc-lab-target.version.json");
        if target_path.exists() {
            records.push(build_info_record(
                run,
                &target_path,
                ToolVersionRole::TargetRunner,
            )?);
        }
        let manifest_path = run.run_dir.join("release-manifest.json");
        if manifest_path.exists() {
            records.push(release_manifest_record(run, &manifest_path)?);
        }
        let doctor_path = run.run_dir.join("privilege/privilege_doctor.json");
        if doctor_path.exists() {
            if let Some(record) = privilege_helper_record(run, &doctor_path)? {
                records.push(record);
            }
        }
    }

    let mut versions = BTreeSet::new();
    let mut git_shas = BTreeSet::new();
    for record in &records {
        if record.version != "unknown" {
            versions.insert(record.version.clone());
        }
        if record.git_sha != "unknown" {
            git_shas.insert(record.git_sha.clone());
        }
    }
    let mut skew_reasons = Vec::new();
    if versions.len() > 1 {
        skew_reasons.push(format!(
            "version mismatch across workflow tools: {}",
            versions.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if git_shas.len() > 1 {
        skew_reasons.push(format!(
            "git_sha mismatch across workflow tools: {}",
            git_shas.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    Ok(RunValidationVersionSet {
        records,
        skew_detected: !skew_reasons.is_empty(),
        skew_reasons,
    })
}

fn build_info_record(
    run: &RunContext,
    path: &Path,
    role: ToolVersionRole,
) -> LabResult<ToolVersionRecord> {
    let info: BuildInfo = read_json(path)?;
    Ok(ToolVersionRecord {
        role,
        tool_name: info.name,
        version: info.version,
        git_sha: info.git_sha,
        target_triple: info.target_triple,
        build_profile: info.build_profile,
        artifact_ref: run.artifact_uri(path)?,
    })
}

fn release_manifest_record(run: &RunContext, path: &Path) -> LabResult<ToolVersionRecord> {
    let manifest: ReleaseManifest = read_json(path)?;
    Ok(ToolVersionRecord {
        role: ToolVersionRole::ReleaseManifest,
        tool_name: "release-manifest".to_string(),
        version: manifest.version,
        git_sha: manifest.git_sha,
        target_triple: manifest.target_triple,
        build_profile: "release".to_string(),
        artifact_ref: run.artifact_uri(path)?,
    })
}

fn privilege_helper_record(run: &RunContext, path: &Path) -> LabResult<Option<ToolVersionRecord>> {
    let report: PrivilegeDoctorReport = read_json(path)?;
    let Some(version) = report.helper_version else {
        return Ok(None);
    };
    Ok(Some(ToolVersionRecord {
        role: ToolVersionRole::PrivilegedHelper,
        tool_name: "adc-lab-priv-helper".to_string(),
        version,
        git_sha: "unknown".to_string(),
        target_triple: "unknown".to_string(),
        build_profile: "unknown".to_string(),
        artifact_ref: run.artifact_uri(path)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::Refusal;
    use crate::control::{new_approval_record, new_cpufreq_plan, refused_result};
    use crate::fsutil::write_json_pretty;
    use crate::probe::{attach_load_control_context, load_artifact_v2};
    use crate::{LoadRestoreOnAbortStatus, LoadSafetyMonitorResult};

    #[test]
    fn fullset_validation_marks_approval_mismatch_refused() {
        let temp = tempfile::tempdir().unwrap();
        let run = test_run(temp.path());
        let plan = new_cpufreq_plan(
            &run,
            &crate::TargetSpec::parse("local").unwrap(),
            "performance".to_string(),
            60,
            Some(75.0),
        );
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.json", plan.plan_id)),
            &plan,
        )
        .unwrap();
        let mut wrong_plan = plan.clone();
        wrong_plan.plan_id = "PLAN-WRONG".to_string();
        let approval =
            new_approval_record(&wrong_plan, "operator".to_string(), "wrong".to_string()).unwrap();
        write_json_pretty(
            run.run_dir
                .join("approvals")
                .join(format!("{}.json", approval.approval_id)),
            &approval,
        )
        .unwrap();
        let refused = refused_result(
            &plan,
            Refusal {
                reason_code: RefusalCode::ApprovalMismatch,
                message: "approval does not match".to_string(),
            },
        );
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", refused.result_id)),
            &refused,
        )
        .unwrap();

        let validation = validate_fullset_run(&run, vec!["performance".to_string()]).unwrap();
        let result = &validation.payload.governor_results[0];
        assert_eq!(result.validity, GovernorValidity::Refused);
        assert!(result.messages[0].contains("approval mismatch"));
        assert!(matches!(validation.status, Status::UnsafeBlocked { .. }));
    }
    #[test]
    fn fullset_validation_marks_unlinked_load_contaminated() {
        let temp = tempfile::tempdir().unwrap();
        let run = test_run(temp.path());
        let target = crate::TargetSpec::parse("local").unwrap();
        let plan = new_cpufreq_plan(&run, &target, "ondemand".to_string(), 60, Some(75.0));
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.json", plan.plan_id)),
            &plan,
        )
        .unwrap();
        let approval =
            new_approval_record(&plan, "operator".to_string(), "approve".to_string()).unwrap();
        write_json_pretty(
            run.run_dir
                .join("approvals")
                .join(format!("{}.json", approval.approval_id)),
            &approval,
        )
        .unwrap();
        let applied = applied_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", applied.result_id)),
            &applied,
        )
        .unwrap();
        let restored = restored_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", restored.result_id)),
            &restored,
        )
        .unwrap();
        write_json_pretty(
            run.run_dir.join("health/restore_health_check.json"),
            &healthy_check(),
        )
        .unwrap();
        let load = load_artifact_v2(run.run_id.clone(), completed_load("local-target"));
        write_json_pretty(run.run_dir.join("load/cpu.LOAD-RESULT-1.v2.json"), &load).unwrap();

        let validation = validate_fullset_run(&run, vec!["ondemand".to_string()]).unwrap();
        let result = &validation.payload.governor_results[0];
        assert_eq!(result.validity, GovernorValidity::Contaminated);
        assert!(result.messages[0].contains("explicit control-result"));
    }
    #[test]
    fn fullset_validation_rejects_load_with_wrong_control_ref() {
        let temp = tempfile::tempdir().unwrap();
        let run = test_run(temp.path());
        let target = crate::TargetSpec::parse("local").unwrap();
        let plan = new_cpufreq_plan(&run, &target, "powersave".to_string(), 60, Some(75.0));
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.json", plan.plan_id)),
            &plan,
        )
        .unwrap();
        let approval =
            new_approval_record(&plan, "operator".to_string(), "approve".to_string()).unwrap();
        write_json_pretty(
            run.run_dir
                .join("approvals")
                .join(format!("{}.json", approval.approval_id)),
            &approval,
        )
        .unwrap();
        let applied = applied_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", applied.result_id)),
            &applied,
        )
        .unwrap();
        let restored = restored_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", restored.result_id)),
            &restored,
        )
        .unwrap();
        write_json_pretty(
            run.run_dir.join("health/restore_health_check.json"),
            &healthy_check(),
        )
        .unwrap();
        let mut load = load_artifact_v2(run.run_id.clone(), completed_load("local-target"));
        attach_load_control_context(
            &mut load,
            Some("artifact://lab/runs/LAB-RUN-001/plans/RESULT-WRONG.result.json".to_string()),
            Some("powersave".to_string()),
        );
        write_json_pretty(run.run_dir.join("load/cpu.LOAD-RESULT-1.v2.json"), &load).unwrap();

        let validation = validate_fullset_run(&run, vec!["powersave".to_string()]).unwrap();
        let result = &validation.payload.governor_results[0];
        assert_eq!(result.validity, GovernorValidity::Contaminated);
    }
    #[test]
    fn fullset_validation_accepts_linked_load_with_restore_and_health() {
        let temp = tempfile::tempdir().unwrap();
        let run = test_run(temp.path());
        let target = crate::TargetSpec::parse("local").unwrap();
        let plan = new_cpufreq_plan(&run, &target, "powersave".to_string(), 60, Some(75.0));
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.json", plan.plan_id)),
            &plan,
        )
        .unwrap();
        let approval =
            new_approval_record(&plan, "operator".to_string(), "approve".to_string()).unwrap();
        write_json_pretty(
            run.run_dir
                .join("approvals")
                .join(format!("{}.json", approval.approval_id)),
            &approval,
        )
        .unwrap();
        let applied = applied_result(&plan);
        let applied_path = run
            .run_dir
            .join("plans")
            .join(format!("{}.result.json", applied.result_id));
        write_json_pretty(&applied_path, &applied).unwrap();
        let control_ref = run.artifact_uri(&applied_path).unwrap();
        let restored = restored_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", restored.result_id)),
            &restored,
        )
        .unwrap();
        write_json_pretty(
            run.run_dir.join("health/restore_health_check.json"),
            &healthy_check(),
        )
        .unwrap();
        let mut load = load_artifact_v2(run.run_id.clone(), completed_load("local-target"));
        attach_load_control_context(&mut load, Some(control_ref), Some("powersave".to_string()));
        write_json_pretty(run.run_dir.join("load/cpu.LOAD-RESULT-1.v2.json"), &load).unwrap();

        let validation = validate_fullset_run(&run, vec!["powersave".to_string()]).unwrap();
        let result = &validation.payload.governor_results[0];
        assert_eq!(result.validity, GovernorValidity::Measured);
        assert!(matches!(validation.status, Status::Measured));
    }

    fn test_run(root: &Path) -> RunContext {
        let run_dir = root.join("LAB-RUN-001");
        fs::create_dir_all(&run_dir).unwrap();
        RunContext {
            run_id: "LAB-RUN-001".to_string(),
            run_dir,
        }
    }

    fn applied_result(plan: &ControlPlan) -> ControlResult {
        ControlResult {
            schema_version: "lab.control_result.v1".to_string(),
            result_id: new_id("RESULT"),
            plan_id: plan.plan_id.clone(),
            target_id: plan.target_id.clone(),
            operation_id: plan.operation.operation_id.clone(),
            risk_tier: plan.risk_tier.clone(),
            status: ControlResultStatus::Applied,
            refusal: None,
            restore_lease: None,
            restore_attempted: false,
            restore_result: None,
            time_unix_ms: now_unix_ms(),
        }
    }

    fn healthy_check() -> HealthCheck {
        HealthCheck {
            schema_version: "lab.health_check.v1".to_string(),
            target_id: "local-target".to_string(),
            status: "ok".to_string(),
            inventory_available: true,
            toolchain_available: true,
        }
    }

    fn restored_result(plan: &ControlPlan) -> ControlResult {
        ControlResult {
            status: ControlResultStatus::Restored,
            restore_attempted: true,
            ..applied_result(plan)
        }
    }

    fn completed_load(target_id: &str) -> crate::LoadResult {
        crate::LoadResult {
            schema_version: "lab.load_result.v1".to_string(),
            result_id: "LOAD-RESULT-1".to_string(),
            load_id: "LOAD-1".to_string(),
            target_id: target_id.to_string(),
            status: "completed".to_string(),
            workers: 1,
            duration_ms: 1000,
            abort_reason: None,
            max_observed_temp_c: Some(55.0),
            worker_iterations: vec![1],
            safety_monitor: LoadSafetyMonitorResult {
                sample_interval_ms: 100,
                samples: 10,
                thermal_surface_available: true,
                operator_abort_observed: false,
                restore_on_abort_status: LoadRestoreOnAbortStatus::NotRequired,
            },
            time_unix_ms: now_unix_ms(),
        }
    }
}
