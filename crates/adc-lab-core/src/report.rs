use crate::contracts::{
    BuildInfo, ClaimDecision, ClaimEvidenceTrace, ClaimTraceEntry, ExperimentRun, ExperimentTrial,
    FamiliarizationPack, OperatingPointBlockedPoint, OperatingPointClaimBoundary,
    OperatingPointCoverage, OperatingPointCoveragePoint, OperatingPointCoverageStatus,
    OperatingPointEvidenceClass, ReleaseManifest, RunArtifactRef, RunDataQuality, RunManifest,
};
use crate::ids::now_unix_ms;
use crate::{artifact_uri_for_run, run_id_from_run_dir, LabResult};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

const READ_ONLY_REQUIRED_AUDIT_OPS: &[&str] = &[
    "inventory",
    "toolchain.discover",
    "tool.qualify_inventory",
    "observe",
];

const OP_INVENTORY: &str = "inventory";
const OP_TOOLCHAIN_DISCOVERY: &str = "toolchain_discovery";
const OP_PASSIVE_OBSERVE: &str = "passive_observe";
const OP_BOUNDED_LOAD: &str = "bounded_load";
const OP_PRIVILEGED_CONTROL: &str = "privileged_control";
const OP_CONTROLLED_OPERATING_POINT: &str = "controlled_operating_point";
const OP_SUSTAINED_THERMAL: &str = "sustained_thermal";
const STATUS_COMPLETED: &str = "completed";
const STATUS_NOT_RUN: &str = "not_run";

#[derive(Debug, Clone)]
struct RunEvidenceSummary {
    run_id: String,
    target_inventory_ref: Option<String>,
    toolchain_inventory_ref: Option<String>,
    tool_qualification_summary_ref: Option<String>,
    observation_ref: Option<String>,
    claim_trace_ref: Option<String>,
    load_result_refs: Vec<String>,
    operations_summary: BTreeMap<String, String>,
    operation_audit_refs: BTreeMap<String, String>,
    audit_event_count: usize,
    audit_run_id_mismatches: Vec<String>,
}

pub fn pack_run(run_dir: impl AsRef<Path>, target_id: String) -> LabResult<FamiliarizationPack> {
    let run_dir = run_dir.as_ref();
    let summary = run_evidence_summary(run_dir)?;
    let artifact_refs = collect_artifact_refs(run_dir, &summary.run_id)?;
    let restore_status = restore_status(run_dir)?;
    Ok(FamiliarizationPack {
        schema_version: "lab.familiarization_pack.v1".to_string(),
        run_id: summary.run_id.clone(),
        target_id,
        pack_status: pack_status(&summary),
        operations_summary: summary.operations_summary.clone(),
        artifact_refs,
        supported_claims: supported_claims_for_summary(&summary),
        blocked_claims: blocked_claims_for_summary(&summary),
        next_evidence_needed: next_evidence_needed_for_summary(&summary),
        audit_event_count: summary.audit_event_count,
        restore_status,
        claim_trace_ref: summary.claim_trace_ref.clone(),
        tool_qualification_summary_ref: summary.tool_qualification_summary_ref.clone(),
        time_unix_ms: now_unix_ms(),
    })
}

pub fn read_only_claim_trace(
    run_dir: impl AsRef<Path>,
    target_id: String,
) -> LabResult<ClaimEvidenceTrace> {
    let run_dir = run_dir.as_ref();
    let summary = run_evidence_summary(run_dir)?;

    Ok(ClaimEvidenceTrace {
        schema_version: "lab.claim_evidence_trace.v1".to_string(),
        run_id: summary.run_id.clone(),
        target_id,
        claims: claim_trace_entries_for_summary(&summary),
        time_unix_ms: now_unix_ms(),
    })
}

pub fn run_manifest(
    run_dir: impl AsRef<Path>,
    target_id: String,
    target: String,
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
    controller_build_info: BuildInfo,
) -> LabResult<RunManifest> {
    let run_dir = run_dir.as_ref();
    let summary = run_evidence_summary(run_dir)?;
    let run_id = summary.run_id.clone();
    let artifacts = known_run_artifacts(run_dir, &run_id)?;
    let audit_ref = artifact_ref_if_exists(run_dir, &run_id, "audit.jsonl")?
        .unwrap_or_else(|| format!("artifact://lab/runs/{run_id}/audit.jsonl"));
    let claim_trace_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/claim_evidence_trace.json")?;
    let identity = release_identity(run_dir, &controller_build_info)?;
    let data_quality = run_data_quality(run_dir, &summary, &identity)?;
    Ok(RunManifest {
        schema_version: "lab.run_manifest.v1".to_string(),
        run_id,
        target_id,
        target,
        mode: run_mode(&summary),
        started_at_unix_ms,
        ended_at_unix_ms,
        adc_lab_version: controller_build_info.version,
        adc_lab_git_sha: controller_build_info.git_sha,
        adc_lab_target_version: identity.adc_lab_target_version,
        adc_lab_target_git_sha: identity.adc_lab_target_git_sha,
        release_tag: identity.release_tag,
        release_asset: identity.release_asset,
        release_asset_sha256: identity.release_asset_sha256,
        binary_sha256: identity.binary_sha256,
        operations_summary: summary.operations_summary,
        operation_audit_refs: summary.operation_audit_refs,
        artifacts,
        audit_ref,
        claim_trace_ref,
        data_quality,
    })
}

pub fn operating_point_coverage(
    run_dir: impl AsRef<Path>,
    target_id: String,
) -> LabResult<OperatingPointCoverage> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let observation_ref = artifact_ref_if_exists(run_dir, &run_id, "observations/observe.json")?;
    let experiment_ref =
        artifact_ref_if_exists(run_dir, &run_id, "experiments/experiment_run.json")?;
    let experiment_run =
        read_experiment_run_if_exists(run_dir.join("experiments/experiment_run.json"))?;

    let mut observed_points = Vec::new();
    if let Some(ref artifact_ref) = observation_ref {
        observed_points.push(OperatingPointCoveragePoint {
            factor_id: "default_policy_frequency".to_string(),
            level: "observed_current_policy".to_string(),
            coverage_status: OperatingPointCoverageStatus::ObservationalOnly,
            evidence_class: OperatingPointEvidenceClass::PassiveObservation,
            evidence_refs: vec![artifact_ref.clone()],
            claim_boundary:
                "Frequency/resource signals were passively observed under the current target policy; this is not a controlled sweep."
                    .to_string(),
        });
    }

    let mut controlled_points = Vec::new();
    let mut controlled_keys = BTreeSet::new();
    let mut blocked_points = Vec::new();
    if let Some(experiment) = experiment_run.as_ref() {
        for trial in &experiment.trials {
            if trial.status == "completed" {
                add_completed_trial_points(
                    trial,
                    experiment_ref.as_ref(),
                    &mut controlled_keys,
                    &mut controlled_points,
                );
            } else if trial.status == "blocked"
                || trial.status == "failed"
                || trial.status == "not_implemented"
            {
                add_blocked_trial_points(trial, &mut blocked_points);
            }
        }
    }

    ensure_fixed_frequency_blocked(&mut blocked_points);
    let coverage_status = coverage_status(&observed_points, &controlled_points, &blocked_points);
    let claim_boundaries = operating_point_claim_boundaries(
        &run_id,
        observation_ref,
        experiment_ref,
        &coverage_status,
        &controlled_points,
        &blocked_points,
    );

    Ok(OperatingPointCoverage {
        schema_version: "lab.operating_point_coverage.v1".to_string(),
        run_id,
        target_id,
        coverage_status,
        observed_points,
        controlled_points,
        blocked_points,
        claim_boundaries,
        time_unix_ms: now_unix_ms(),
    })
}

fn read_experiment_run_if_exists(path: PathBuf) -> LabResult<Option<ExperimentRun>> {
    read_json_artifact_if_exists(path)
}

fn add_completed_trial_points(
    trial: &ExperimentTrial,
    experiment_ref: Option<&String>,
    controlled_keys: &mut BTreeSet<(String, String)>,
    controlled_points: &mut Vec<OperatingPointCoveragePoint>,
) {
    for (factor_id, level) in &trial.factors {
        if factor_id != "cpu_load_workers" {
            continue;
        }
        let key = (factor_id.clone(), level.clone());
        if !controlled_keys.insert(key) {
            continue;
        }
        let mut evidence_refs = Vec::new();
        if let Some(reference) = experiment_ref {
            evidence_refs.push(reference.clone());
        }
        evidence_refs.extend(trial.artifact_refs.clone());
        controlled_points.push(OperatingPointCoveragePoint {
            factor_id: factor_id.clone(),
            level: level.clone(),
            coverage_status: OperatingPointCoverageStatus::ControlledSubset,
            evidence_class: OperatingPointEvidenceClass::BoundedLoad,
            evidence_refs,
            claim_boundary:
                "Workload intensity was controlled for this bounded trial; CPU frequency remains observed under current policy unless privileged frequency-control evidence exists."
                    .to_string(),
        });
    }
}

fn add_blocked_trial_points(
    trial: &ExperimentTrial,
    blocked_points: &mut Vec<OperatingPointBlockedPoint>,
) {
    for (factor_id, level) in &trial.factors {
        if factor_id == "cpu_load_workers" && trial.status != "failed" {
            continue;
        }
        let coverage_status = blocked_status_for_factor(factor_id);
        blocked_points.push(OperatingPointBlockedPoint {
            factor_id: factor_id.clone(),
            requested_level: Some(level.clone()),
            coverage_status,
            reason: trial
                .failure
                .clone()
                .unwrap_or_else(|| format!("trial ended with status {}", trial.status)),
            next_evidence_needed: next_evidence_for_blocked_factor(factor_id),
        });
    }
}

fn ensure_fixed_frequency_blocked(blocked_points: &mut Vec<OperatingPointBlockedPoint>) {
    if blocked_points
        .iter()
        .any(|point| point.factor_id == "fixed_cpu_frequency")
    {
        return;
    }
    blocked_points.push(OperatingPointBlockedPoint {
        factor_id: "fixed_cpu_frequency".to_string(),
        requested_level: Some("all_fixed_cpu_frequencies".to_string()),
        coverage_status: OperatingPointCoverageStatus::NotControllable,
        reason: "observed frequency variation is not a controlled fixed-frequency sweep"
            .to_string(),
        next_evidence_needed: vec![
            "approved privileged frequency control".to_string(),
            "controlled operating point matrix with fixed-frequency levels".to_string(),
            "restore verification for every controlled point".to_string(),
        ],
    });
}

fn blocked_status_for_factor(factor_id: &str) -> OperatingPointCoverageStatus {
    if is_safety_blocked_factor(factor_id) {
        OperatingPointCoverageStatus::BlockedUnsafe
    } else {
        OperatingPointCoverageStatus::NotControllable
    }
}

fn is_safety_blocked_factor(factor_id: &str) -> bool {
    [
        "thermal_stress",
        "battery_drain",
        "filesystem_pressure",
        "storage_pressure",
        "watchdog_reboot",
        "memory_pressure_near_oom",
    ]
    .iter()
    .any(|needle| factor_id.contains(needle))
}

fn next_evidence_for_blocked_factor(factor_id: &str) -> Vec<String> {
    if is_safety_blocked_factor(factor_id) {
        return vec![
            "explicit risk boundary".to_string(),
            "operator approval".to_string(),
            "abort condition and recovery plan".to_string(),
        ];
    }
    if factor_id == "governor" || factor_id == "fixed_cpu_frequency" {
        return vec![
            "approved privileged control plan".to_string(),
            "plan/apply/restore integration in matrix runner".to_string(),
            "restore verification evidence".to_string(),
        ];
    }
    vec!["implemented controlled-factor runner support".to_string()]
}

fn coverage_status(
    observed_points: &[OperatingPointCoveragePoint],
    controlled_points: &[OperatingPointCoveragePoint],
    blocked_points: &[OperatingPointBlockedPoint],
) -> OperatingPointCoverageStatus {
    if blocked_points
        .iter()
        .any(|point| point.coverage_status == OperatingPointCoverageStatus::BlockedUnsafe)
    {
        OperatingPointCoverageStatus::BlockedUnsafe
    } else if !controlled_points.is_empty() {
        OperatingPointCoverageStatus::ControlledSubset
    } else if !observed_points.is_empty() {
        OperatingPointCoverageStatus::ObservationalOnly
    } else {
        OperatingPointCoverageStatus::NotControllable
    }
}

fn operating_point_claim_boundaries(
    run_id: &str,
    observation_ref: Option<String>,
    experiment_ref: Option<String>,
    coverage_status: &OperatingPointCoverageStatus,
    controlled_points: &[OperatingPointCoveragePoint],
    blocked_points: &[OperatingPointBlockedPoint],
) -> Vec<OperatingPointClaimBoundary> {
    let coverage_ref =
        format!("artifact://lab/runs/{run_id}/reports/operating_point_coverage.json");
    let mut boundaries = Vec::new();
    boundaries.push(OperatingPointClaimBoundary {
        claim: "frequency/resource variation was observed under the current target policy"
            .to_string(),
        decision: if observation_ref.is_some() {
            ClaimDecision::Provisional
        } else {
            ClaimDecision::Blocked
        },
        evidence_refs: observation_ref.into_iter().collect(),
        next_evidence_needed: vec![
            "controlled operating point matrix for stronger operating-point claims".to_string(),
        ],
    });
    boundaries.push(OperatingPointClaimBoundary {
        claim: "bounded workload operating points were measured for completed trials".to_string(),
        decision: if controlled_points.is_empty() {
            ClaimDecision::Blocked
        } else {
            ClaimDecision::Supported
        },
        evidence_refs: experiment_ref
            .into_iter()
            .chain([coverage_ref.clone()])
            .collect(),
        next_evidence_needed: if controlled_points.is_empty() {
            vec!["execute a supported controlled workload matrix".to_string()]
        } else {
            vec!["add privileged control wiring for CPU frequency/governor factors".to_string()]
        },
    });
    boundaries.push(OperatingPointClaimBoundary {
        claim: "adc-lab verified behavior across all fixed CPU frequencies".to_string(),
        decision: ClaimDecision::Blocked,
        evidence_refs: Vec::new(),
        next_evidence_needed: vec![
            "controlled fixed-frequency matrix".to_string(),
            "approved privileged frequency control".to_string(),
            "restore verification per point".to_string(),
        ],
    });
    boundaries.push(OperatingPointClaimBoundary {
        claim: "coverage status supports production physical-footprint conclusions".to_string(),
        decision: ClaimDecision::Blocked,
        evidence_refs: vec![coverage_ref],
        next_evidence_needed: vec![
            "target-specific sustained thermal evidence".to_string(),
            "observer-effect calibration".to_string(),
            "battery, flash, wakeup, and latency budgets".to_string(),
        ],
    });
    if *coverage_status == OperatingPointCoverageStatus::BlockedUnsafe {
        boundaries.push(OperatingPointClaimBoundary {
            claim: "unsafe or degradation-inducing operating points were executed".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "explicit Tier 3 approval and recovery plan before execution".to_string(),
            ],
        });
    }
    if !blocked_points.is_empty() {
        boundaries.push(OperatingPointClaimBoundary {
            claim: format!(
                "{} operating point(s) are blocked and cannot support claims",
                blocked_points.len()
            ),
            decision: ClaimDecision::Blocked,
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "inspect blocked_points reasons and collect the listed next evidence".to_string(),
            ],
        });
    }
    boundaries
}

fn read_json_artifact_if_exists<T: DeserializeOwned>(path: PathBuf) -> LabResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::LabError::Validation(format!(
            "failed to parse JSON artifact {}: {error}",
            path.display()
        ))
    })?;
    Ok(Some(value))
}

fn load_result_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    Ok(collect_artifact_refs(run_dir, run_id)?
        .into_iter()
        .filter(|artifact| {
            artifact.ends_with("/load_result.json")
                || (artifact.contains("/loads/") && artifact.ends_with(".result.json"))
        })
        .collect())
}

fn collect_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_files(run_dir, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| artifact_uri_for_run(run_id, run_dir, path))
        .collect()
}

fn run_id_from_dir(run_dir: &Path) -> String {
    run_id_from_run_dir(run_dir)
}

fn artifact_ref_if_exists(
    run_dir: &Path,
    run_id: &str,
    relative_path: &str,
) -> LabResult<Option<String>> {
    let path = run_dir.join(relative_path);
    if path.exists() {
        artifact_uri_for_run(run_id, run_dir, path).map(Some)
    } else {
        Ok(None)
    }
}

fn known_run_artifacts(run_dir: &Path, run_id: &str) -> LabResult<Vec<RunArtifactRef>> {
    let mut artifacts = Vec::new();
    for (name, relative_path, schema_version) in [
        ("run_context", "run_context.json", "lab.run_context.v1"),
        (
            "target_inventory",
            "inventory/target_inventory.json",
            "lab.target_inventory.v1",
        ),
        (
            "toolchain_inventory",
            "toolchain/toolchain_inventory.json",
            "lab.toolchain_inventory.v1",
        ),
        (
            "passive_observe",
            "observations/observe.json",
            "lab.observation_result.v1",
        ),
        (
            "claim_evidence_trace",
            "reports/claim_evidence_trace.json",
            "lab.claim_evidence_trace.v1",
        ),
        (
            "tool_qualification_summary",
            "tools/tool_qualification_summary.json",
            "lab.tool_qualification_summary.v1",
        ),
        (
            "adc_lab_version",
            "tools/adc-lab.version.json",
            "lab.build_info.v1",
        ),
        (
            "adc_lab_target_version",
            "tools/adc-lab-target.version.json",
            "lab.build_info.v1",
        ),
        ("audit_log", "audit.jsonl", "lab.audit_event.v1"),
    ] {
        if let Some(artifact_ref) = artifact_ref_if_exists(run_dir, run_id, relative_path)? {
            artifacts.push(RunArtifactRef {
                name: name.to_string(),
                artifact_ref,
                schema_version: schema_version.to_string(),
            });
        }
    }
    Ok(artifacts)
}

fn run_evidence_summary(run_dir: &Path) -> LabResult<RunEvidenceSummary> {
    let run_id = run_id_from_dir(run_dir);
    let target_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let toolchain_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "toolchain/toolchain_inventory.json")?;
    let tool_qualification_summary_ref =
        artifact_ref_if_exists(run_dir, &run_id, "tools/tool_qualification_summary.json")?;
    let observation_ref = artifact_ref_if_exists(run_dir, &run_id, "observations/observe.json")?;
    let claim_trace_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/claim_evidence_trace.json")?;
    let load_result_refs = load_result_artifact_refs(run_dir, &run_id)?;
    let control_result_refs = control_result_artifact_refs(run_dir, &run_id)?;
    let audit_facts = audit_facts(run_dir.join("audit.jsonl"), &run_id)?;

    let mut operations_summary = BTreeMap::new();
    operations_summary.insert(
        OP_INVENTORY.to_string(),
        status_if(target_inventory_ref.is_some()),
    );
    operations_summary.insert(
        OP_TOOLCHAIN_DISCOVERY.to_string(),
        status_if(toolchain_inventory_ref.is_some()),
    );
    operations_summary.insert(
        OP_PASSIVE_OBSERVE.to_string(),
        status_if(observation_ref.is_some()),
    );
    operations_summary.insert(
        OP_BOUNDED_LOAD.to_string(),
        status_if(!load_result_refs.is_empty()),
    );
    operations_summary.insert(
        OP_PRIVILEGED_CONTROL.to_string(),
        status_if(!control_result_refs.is_empty()),
    );
    operations_summary.insert(
        OP_CONTROLLED_OPERATING_POINT.to_string(),
        status_if(experiment_has_completed_trial(
            run_dir.join("experiments/experiment_run.json"),
        )?),
    );
    operations_summary.insert(OP_SUSTAINED_THERMAL.to_string(), STATUS_NOT_RUN.to_string());

    let audit_ref = format!("artifact://lab/runs/{run_id}/audit.jsonl");
    let mut operation_audit_refs = BTreeMap::new();
    for operation in audit_facts.operations {
        operation_audit_refs
            .entry(operation)
            .or_insert(audit_ref.clone());
    }

    Ok(RunEvidenceSummary {
        run_id,
        target_inventory_ref,
        toolchain_inventory_ref,
        tool_qualification_summary_ref,
        observation_ref,
        claim_trace_ref,
        load_result_refs,
        operations_summary,
        operation_audit_refs,
        audit_event_count: audit_facts.event_count,
        audit_run_id_mismatches: audit_facts.run_id_mismatches,
    })
}

#[derive(Debug)]
struct AuditFacts {
    operations: Vec<String>,
    event_count: usize,
    run_id_mismatches: Vec<String>,
}

fn audit_facts(path: PathBuf, expected_run_id: &str) -> LabResult<AuditFacts> {
    if !path.exists() {
        return Ok(AuditFacts {
            operations: Vec::new(),
            event_count: 0,
            run_id_mismatches: Vec::new(),
        });
    }
    let file = fs::File::open(path)?;
    let mut operations = Vec::new();
    let mut event_count = 0;
    let mut run_id_mismatches = Vec::new();
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        event_count += 1;
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if let Some(operation) = value.get("operation").and_then(|value| value.as_str()) {
            operations.push(operation.to_string());
        }
        if let Some(run_id) = value.get("run_id").and_then(|value| value.as_str()) {
            if run_id != expected_run_id {
                run_id_mismatches.push(format!(
                    "audit event line {} run_id {run_id} does not match manifest run_id {expected_run_id}",
                    index + 1
                ));
            }
        }
    }
    Ok(AuditFacts {
        operations,
        event_count,
        run_id_mismatches,
    })
}

fn status_if(completed: bool) -> String {
    if completed {
        STATUS_COMPLETED.to_string()
    } else {
        STATUS_NOT_RUN.to_string()
    }
}

fn experiment_has_completed_trial(path: PathBuf) -> LabResult<bool> {
    let Some(experiment): Option<ExperimentRun> = read_json_artifact_if_exists(path)? else {
        return Ok(false);
    };
    Ok(experiment
        .trials
        .iter()
        .any(|trial| trial.status == "completed"))
}

fn control_result_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    Ok(collect_artifact_refs(run_dir, run_id)?
        .into_iter()
        .filter(|artifact| artifact.contains("/plans/") && artifact.ends_with(".result.json"))
        .collect())
}

fn run_data_quality(
    run_dir: &Path,
    summary: &RunEvidenceSummary,
    identity: &ReleaseIdentity,
) -> LabResult<RunDataQuality> {
    let mut missing = Vec::new();
    let mut inconsistent = Vec::new();
    let mut notes = Vec::new();

    for (label, present) in [
        (
            "target inventory artifact missing",
            summary.target_inventory_ref.is_some(),
        ),
        (
            "toolchain inventory artifact missing",
            summary.toolchain_inventory_ref.is_some(),
        ),
        (
            "passive observation artifact missing",
            summary.observation_ref.is_some(),
        ),
        (
            "tool qualification summary artifact missing",
            summary.tool_qualification_summary_ref.is_some(),
        ),
    ] {
        if !present {
            missing.push(label.to_string());
        }
    }

    for required in READ_ONLY_REQUIRED_AUDIT_OPS {
        if !summary.operation_audit_refs.contains_key(*required) {
            missing.push(format!("audit event missing for {required}"));
        }
    }
    if !summary.load_result_refs.is_empty()
        && !summary.operation_audit_refs.contains_key("load.cpu")
        && !summary
            .operation_audit_refs
            .contains_key("experiment.trial")
    {
        inconsistent.push(
            "bounded load artifact exists without load.cpu or experiment.trial audit event"
                .to_string(),
        );
    }
    if !summary.audit_run_id_mismatches.is_empty() {
        inconsistent.extend(summary.audit_run_id_mismatches.clone());
    }

    if operation_status(summary, OP_CONTROLLED_OPERATING_POINT) == STATUS_NOT_RUN {
        missing.push("controlled operating point experiment was not run".to_string());
    }
    if operation_status(summary, OP_PRIVILEGED_CONTROL) == STATUS_NOT_RUN {
        missing.push("privileged control operation was not run".to_string());
    }
    if operation_status(summary, OP_SUSTAINED_THERMAL) == STATUS_NOT_RUN {
        missing.push("sustained thermal and recovery envelope were not measured".to_string());
    }
    missing.push("wakeups were not measured".to_string());

    if !summary.load_result_refs.is_empty() {
        notes.push("bounded non-root CPU load short-smoke was run".to_string());
    }

    if identity.release_asset_sha256 == "unknown" {
        missing.push("release asset sha256 was not recorded".to_string());
    }
    if identity.adc_lab_target_version == "unknown" {
        missing.push("adc-lab-target version was not recorded".to_string());
    }
    if identity.adc_lab_target_version != "unknown"
        && identity.adc_lab_target_version != identity.adc_lab_version
    {
        inconsistent.push(format!(
            "adc-lab version {} does not match adc-lab-target version {}",
            identity.adc_lab_version, identity.adc_lab_target_version
        ));
    }
    if identity.adc_lab_target_git_sha != "unknown"
        && identity.adc_lab_target_git_sha != identity.adc_lab_git_sha
    {
        inconsistent.push(format!(
            "adc-lab git_sha {} does not match adc-lab-target git_sha {}",
            identity.adc_lab_git_sha, identity.adc_lab_target_git_sha
        ));
    }

    if release_manifest_version_mismatch(run_dir, identity)? {
        inconsistent.push(
            "release manifest version/git_sha does not match adc-lab binary build identity"
                .to_string(),
        );
    }

    Ok(RunDataQuality {
        missing,
        inconsistent,
        notes,
    })
}

fn operation_status<'a>(summary: &'a RunEvidenceSummary, operation: &str) -> &'a str {
    summary
        .operations_summary
        .get(operation)
        .map(String::as_str)
        .unwrap_or(STATUS_NOT_RUN)
}

#[derive(Debug, Clone)]
struct ReleaseIdentity {
    adc_lab_version: String,
    adc_lab_git_sha: String,
    adc_lab_target_version: String,
    adc_lab_target_git_sha: String,
    release_tag: String,
    release_asset: String,
    release_asset_sha256: String,
    binary_sha256: BTreeMap<String, String>,
}

fn release_identity(
    run_dir: &Path,
    controller_build_info: &BuildInfo,
) -> LabResult<ReleaseIdentity> {
    let target_build_info: Option<BuildInfo> =
        read_json_artifact_if_exists(run_dir.join("tools/adc-lab-target.version.json"))?;
    let release_manifest = read_release_manifest(run_dir)?;
    let release_tag = std::env::var("ADC_LAB_RELEASE_TAG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("v{}", controller_build_info.version));
    let release_asset = std::env::var("ADC_LAB_RELEASE_ASSET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "adc-lab-v{}-{}.tar.gz",
                controller_build_info.version,
                asset_triple(&controller_build_info.target_triple)
            )
        });
    let release_asset_sha256 = std::env::var("ADC_LAB_RELEASE_ASSET_SHA256")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let mut binary_sha256 = BTreeMap::new();
    binary_sha256.insert(
        "adc-lab".to_string(),
        current_exe_sha256().unwrap_or_else(|| "unknown".to_string()),
    );
    if let Some(manifest) = release_manifest.as_ref() {
        for binary in &manifest.binaries {
            binary_sha256.insert(binary.name.clone(), binary.sha256.clone());
        }
    }
    binary_sha256
        .entry("adc-lab-target".to_string())
        .or_insert_with(|| "unknown".to_string());

    Ok(ReleaseIdentity {
        adc_lab_version: controller_build_info.version.clone(),
        adc_lab_git_sha: controller_build_info.git_sha.clone(),
        adc_lab_target_version: target_build_info
            .as_ref()
            .map(|info| info.version.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        adc_lab_target_git_sha: target_build_info
            .as_ref()
            .map(|info| info.git_sha.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        release_tag,
        release_asset,
        release_asset_sha256,
        binary_sha256,
    })
}

fn read_release_manifest(run_dir: &Path) -> LabResult<Option<ReleaseManifest>> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("ADC_LAB_RELEASE_MANIFEST") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }
    candidates.push(run_dir.join("release-manifest.json"));
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent().and_then(Path::parent) {
            candidates.push(parent.join("release-manifest.json"));
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return read_json_artifact_if_exists(candidate);
        }
    }
    Ok(None)
}

fn release_manifest_version_mismatch(
    run_dir: &Path,
    identity: &ReleaseIdentity,
) -> LabResult<bool> {
    let Some(manifest) = read_release_manifest(run_dir)? else {
        return Ok(false);
    };
    Ok(
        manifest.version != identity.adc_lab_version
            || manifest.git_sha != identity.adc_lab_git_sha,
    )
}

fn current_exe_sha256() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    sha256_file(&path).ok()
}

fn sha256_file(path: &Path) -> LabResult<String> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn asset_triple(target_triple: &str) -> String {
    match target_triple {
        "aarch64-unknown-linux-gnu" => "linux-aarch64".to_string(),
        "x86_64-unknown-linux-gnu" => "linux-x86_64".to_string(),
        other => other.to_string(),
    }
}

fn claim_entry(
    claim: &str,
    artifact_ref: Option<String>,
    evidence_needed: &str,
) -> ClaimTraceEntry {
    let is_supported = artifact_ref.is_some();
    ClaimTraceEntry {
        claim: claim.to_string(),
        decision: if is_supported {
            ClaimDecision::Supported
        } else {
            ClaimDecision::Blocked
        },
        evidence_refs: artifact_ref.into_iter().collect(),
        next_evidence_needed: if is_supported {
            Vec::new()
        } else {
            vec![evidence_needed.to_string()]
        },
    }
}

fn pack_status(summary: &RunEvidenceSummary) -> String {
    let has_core = summary.target_inventory_ref.is_some()
        && summary.toolchain_inventory_ref.is_some()
        && summary.observation_ref.is_some();
    if !has_core {
        "incomplete".to_string()
    } else if operation_status(summary, OP_CONTROLLED_OPERATING_POINT) == STATUS_COMPLETED {
        "controlled_operating_point_subset".to_string()
    } else if operation_status(summary, OP_BOUNDED_LOAD) == STATUS_COMPLETED {
        "exploratory_short_smoke".to_string()
    } else {
        "observational_read_only".to_string()
    }
}

fn run_mode(summary: &RunEvidenceSummary) -> String {
    if operation_status(summary, OP_BOUNDED_LOAD) == STATUS_COMPLETED {
        "exploratory_short_smoke".to_string()
    } else {
        "read_only_familiarization".to_string()
    }
}

fn supported_claims_for_summary(summary: &RunEvidenceSummary) -> Vec<String> {
    let mut claims = Vec::new();
    if summary.target_inventory_ref.is_some() {
        claims.push("target inventory was collected through read-only surfaces".to_string());
    }
    if summary.toolchain_inventory_ref.is_some() {
        claims.push("toolchain availability was collected through read-only discovery".to_string());
    }
    if summary.tool_qualification_summary_ref.is_some() {
        claims.push("tool qualification summary was generated".to_string());
    }
    if summary.observation_ref.is_some() {
        claims.push("observed covariates were sampled under the current target policy".to_string());
    }
    if !summary.load_result_refs.is_empty() {
        claims.push(
            "target completed bounded CPU load short-smoke under configured safety bounds"
                .to_string(),
        );
    }
    claims
}

fn blocked_claims_for_summary(summary: &RunEvidenceSummary) -> Vec<String> {
    let mut claims = vec![
        "low overhead across all operating points".to_string(),
        "battery safe".to_string(),
        "fixed CPU frequency behavior".to_string(),
        "production readiness".to_string(),
        "no observer effect".to_string(),
        "all operating points measured".to_string(),
        "thermally safe for sustained production load".to_string(),
    ];
    if summary.load_result_refs.is_empty() {
        claims.push("bounded workload completion".to_string());
    }
    claims
}

fn next_evidence_needed_for_summary(summary: &RunEvidenceSummary) -> Vec<String> {
    let mut needed = vec![
        "controlled operating point matrix".to_string(),
        "tool qualification for privileged control".to_string(),
        "target-specific resource budget calibration".to_string(),
        "repeat trials".to_string(),
        "sustained thermal run".to_string(),
        "wakeups, battery/power, storage/write, and latency/jitter measurement".to_string(),
    ];
    if summary.load_result_refs.is_empty() {
        needed.push("bounded load with safety monitor".to_string());
    }
    needed
}

fn claim_trace_entries_for_summary(summary: &RunEvidenceSummary) -> Vec<ClaimTraceEntry> {
    let mut claims = vec![
        claim_entry(
            "target identity was observed through read-only inventory surfaces",
            summary.target_inventory_ref.clone(),
            "read-only target inventory",
        ),
        claim_entry(
            "toolchain availability was observed through read-only discovery",
            summary.toolchain_inventory_ref.clone(),
            "read-only toolchain inventory",
        ),
        claim_entry(
            "tool qualification summary was generated for discovered toolchain",
            summary.tool_qualification_summary_ref.clone(),
            "tool qualification summary",
        ),
        ClaimTraceEntry {
            claim: "passive resource signals were sampled under the current target policy"
                .to_string(),
            decision: if summary.observation_ref.is_some() {
                ClaimDecision::Provisional
            } else {
                ClaimDecision::Blocked
            },
            evidence_refs: summary.observation_ref.clone().into_iter().collect(),
            next_evidence_needed: vec![
                "controlled operating point matrix".to_string(),
                "observer effect calibration".to_string(),
            ],
        },
    ];

    if !summary.load_result_refs.is_empty() {
        claims.push(ClaimTraceEntry {
            claim:
                "target completed bounded 2-worker CPU 60s short-smoke under configured thermal abort"
                    .to_string(),
            decision: ClaimDecision::Supported,
            evidence_refs: summary.load_result_refs.clone(),
            next_evidence_needed: vec![
                "repeat trials".to_string(),
                "sustained thermal run".to_string(),
                "controlled operating point matrix".to_string(),
            ],
        });
    } else {
        claims.push(ClaimTraceEntry {
            claim: "target completed bounded CPU load short-smoke under configured thermal abort"
                .to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: Vec::new(),
            next_evidence_needed: vec!["bounded CPU load result artifact".to_string()],
        });
    }

    claims.extend([
        ClaimTraceEntry {
            claim: "fixed CPU frequency behavior was verified".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: vec![],
            next_evidence_needed: vec![
                "approved privileged control plan".to_string(),
                "controlled operating point matrix".to_string(),
            ],
        },
        ClaimTraceEntry {
            claim: "target is thermally safe for sustained production load".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: vec![],
            next_evidence_needed: vec![
                "30min sustained thermal run".to_string(),
                "cooldown/recovery curve".to_string(),
                "repeated trials".to_string(),
            ],
        },
        ClaimTraceEntry {
            claim:
                "target runtime is production-ready, battery-safe, flash-safe, low-overhead, and thermally safe"
                    .to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: vec![],
            next_evidence_needed: vec![
                "target-specific resource budgets".to_string(),
                "sustained thermal and observer-effect evidence".to_string(),
                "wakeups, battery/power, storage/write, and latency/jitter evidence".to_string(),
            ],
        },
    ]);
    claims
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) -> LabResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(crate::LabError::Validation(format!(
            "artifact collection refuses symlink: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        collect_files(&entry.path(), out)?;
    }
    Ok(())
}

fn restore_status(run_dir: &Path) -> LabResult<String> {
    let lease_dir = run_dir.join("leases");
    if !lease_dir.exists() {
        return Ok("not_required".to_string());
    }
    let has_lease = fs::read_dir(lease_dir)?.flatten().next().is_some();
    Ok(if has_lease {
        "pending_or_recorded".to_string()
    } else {
        "not_required".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_validation_pack_uses_logical_artifact_refs() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        let report_dir = run_dir.join("reports");
        fs::create_dir_all(&report_dir).unwrap();
        fs::write(run_dir.join("audit.jsonl"), "{}\n").unwrap();
        fs::write(report_dir.join("claim_evidence_trace.json"), "{}").unwrap();

        let pack = pack_run(&run_dir, "local-target".to_string()).unwrap();
        assert!(pack
            .artifact_refs
            .iter()
            .all(|artifact| artifact.starts_with("artifact://lab/runs/LAB-RUN-001/")));
        assert!(pack
            .artifact_refs
            .iter()
            .all(|artifact| !artifact.contains(temp.path().to_str().unwrap())));
    }

    #[test]
    fn contract_validation_read_only_manifest_records_missing_control_claims() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::create_dir_all(run_dir.join("toolchain")).unwrap();
        fs::create_dir_all(run_dir.join("observations")).unwrap();
        fs::create_dir_all(run_dir.join("reports")).unwrap();
        fs::write(run_dir.join("inventory/target_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("toolchain/toolchain_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("observations/observe.json"), "{}").unwrap();
        fs::write(
            run_dir.join("audit.jsonl"),
            [
                r#"{"operation":"inventory"}"#,
                r#"{"operation":"toolchain.discover"}"#,
                r#"{"operation":"observe"}"#,
            ]
            .join("\n"),
        )
        .unwrap();

        let trace = read_only_claim_trace(&run_dir, "local-target".to_string()).unwrap();
        assert!(trace
            .claims
            .iter()
            .any(|claim| claim.decision == ClaimDecision::Blocked
                && claim.claim.contains("fixed CPU frequency")));

        fs::write(
            run_dir.join("reports/claim_evidence_trace.json"),
            serde_json::to_vec(&trace).unwrap(),
        )
        .unwrap();
        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert!(manifest
            .artifacts
            .iter()
            .all(|artifact| artifact.artifact_ref.starts_with("artifact://lab/runs/")));
        assert!(manifest
            .data_quality
            .missing
            .contains(&"controlled operating point experiment was not run".to_string()));
    }

    #[test]
    fn contract_validation_load_artifact_updates_manifest_pack_and_claim_trace() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        fs::create_dir_all(run_dir.join("loads")).unwrap();
        fs::write(
            run_dir.join("loads/LOAD-001.result.json"),
            serde_json::json!({
                "schema_version": "lab.load_result.v1",
                "result_id": "LOAD-RESULT-001",
                "load_id": "LOAD-001",
                "target_id": "local-target",
                "status": "completed",
                "workers": 2,
                "duration_ms": 60000,
                "abort_reason": null,
                "max_observed_temp_c": 54.5,
                "worker_iterations": [1, 2],
                "safety_monitor": {
                    "sample_interval_ms": 100,
                    "samples": 600,
                    "thermal_surface_available": true,
                    "operator_abort_observed": false,
                    "restore_on_abort_status": "not_required"
                },
                "time_unix_ms": 3
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            run_dir.join("audit.jsonl"),
            [
                audit_line("LAB-RUN-001", "inventory"),
                audit_line("LAB-RUN-001", "toolchain.discover"),
                audit_line("LAB-RUN-001", "tool.qualify_inventory"),
                audit_line("LAB-RUN-001", "observe"),
                audit_line("LAB-RUN-001", "load.cpu"),
            ]
            .join("\n"),
        )
        .unwrap();

        let trace = read_only_claim_trace(&run_dir, "local-target".to_string()).unwrap();
        assert!(trace.claims.iter().any(|claim| {
            claim.decision == ClaimDecision::Supported
                && claim.claim.contains("bounded 2-worker CPU 60s short-smoke")
                && claim
                    .evidence_refs
                    .iter()
                    .all(|artifact| artifact.starts_with("artifact://lab/runs/LAB-RUN-001/"))
        }));
        assert!(trace.claims.iter().any(|claim| {
            claim.decision == ClaimDecision::Blocked
                && claim.claim.contains("sustained production load")
                && claim.evidence_refs.is_empty()
        }));

        let pack = pack_run(&run_dir, "local-target".to_string()).unwrap();
        assert_ne!(pack.pack_status, "observational_read_only");
        assert_eq!(pack.pack_status, "exploratory_short_smoke");
        assert_eq!(
            pack.operations_summary
                .get(OP_BOUNDED_LOAD)
                .map(String::as_str),
            Some(STATUS_COMPLETED)
        );
        assert!(pack.tool_qualification_summary_ref.is_some());

        fs::create_dir_all(run_dir.join("reports")).unwrap();
        fs::write(
            run_dir.join("reports/claim_evidence_trace.json"),
            serde_json::to_vec(&trace).unwrap(),
        )
        .unwrap();
        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert_eq!(
            manifest
                .operations_summary
                .get(OP_BOUNDED_LOAD)
                .map(String::as_str),
            Some(STATUS_COMPLETED)
        );
        assert!(!manifest
            .data_quality
            .missing
            .iter()
            .any(|item| item.contains("no load") || item.contains("load or stress")));
        assert!(manifest
            .data_quality
            .notes
            .contains(&"bounded non-root CPU load short-smoke was run".to_string()));
    }

    #[test]
    fn contract_validation_audit_run_id_mismatch_degrades_manifest_quality() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        fs::write(
            run_dir.join("audit.jsonl"),
            [
                audit_line("LAB-RUN-OTHER", "inventory"),
                audit_line("LAB-RUN-001", "toolchain.discover"),
                audit_line("LAB-RUN-001", "tool.qualify_inventory"),
                audit_line("LAB-RUN-001", "observe"),
            ]
            .join("\n"),
        )
        .unwrap();

        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert!(manifest
            .data_quality
            .inconsistent
            .iter()
            .any(|item| { item.contains("LAB-RUN-OTHER") && item.contains("LAB-RUN-001") }));
    }

    #[test]
    fn contract_validation_operating_point_coverage_separates_passive_observation() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("observations")).unwrap();
        fs::write(run_dir.join("observations/observe.json"), "{}").unwrap();

        let coverage = operating_point_coverage(&run_dir, "local-target".to_string()).unwrap();
        assert_eq!(
            coverage.coverage_status,
            OperatingPointCoverageStatus::ObservationalOnly
        );
        assert_eq!(coverage.observed_points.len(), 1);
        assert!(coverage.controlled_points.is_empty());
        assert!(coverage
            .blocked_points
            .iter()
            .any(|point| point.factor_id == "fixed_cpu_frequency"));
        assert!(coverage
            .claim_boundaries
            .iter()
            .any(|boundary| boundary.claim.contains("fixed CPU frequencies")
                && boundary.decision == ClaimDecision::Blocked));
    }

    #[test]
    fn contract_validation_operating_point_coverage_records_controlled_subset() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("experiments")).unwrap();
        fs::write(
            run_dir.join("experiments/experiment_run.json"),
            serde_json::json!({
                "schema_version": "lab.experiment_run.v1",
                "run_id": "LAB-RUN-001",
                "matrix_id": "MATRIX-LOAD",
                "target_id": "local-target",
                "dry_run": false,
                "trials": [
                    {
                        "trial_id": "TRIAL-001",
                        "factors": { "cpu_load_workers": "1" },
                        "status": "completed",
                        "artifact_refs": [
                            "artifact://lab/runs/LAB-RUN-001/experiments/trials/TRIAL-001/load_result.json",
                            "artifact://lab/runs/LAB-RUN-001/experiments/trials/TRIAL-001/observation.json"
                        ],
                        "failure": null,
                        "started_at_unix_ms": 1,
                        "ended_at_unix_ms": 2
                    }
                ],
                "time_unix_ms": 3
            })
            .to_string(),
        )
        .unwrap();

        let coverage = operating_point_coverage(&run_dir, "local-target".to_string()).unwrap();
        assert_eq!(
            coverage.coverage_status,
            OperatingPointCoverageStatus::ControlledSubset
        );
        assert!(coverage.observed_points.is_empty());
        assert_eq!(coverage.controlled_points.len(), 1);
        assert_eq!(coverage.controlled_points[0].factor_id, "cpu_load_workers");
        assert_eq!(
            coverage.controlled_points[0].coverage_status,
            OperatingPointCoverageStatus::ControlledSubset
        );
        assert!(coverage
            .claim_boundaries
            .iter()
            .any(|boundary| boundary.claim.contains("bounded workload")
                && boundary.decision == ClaimDecision::Supported));
    }

    #[test]
    fn contract_validation_operating_point_coverage_records_blocked_factor() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("experiments")).unwrap();
        fs::write(
            run_dir.join("experiments/experiment_run.json"),
            serde_json::json!({
                "schema_version": "lab.experiment_run.v1",
                "run_id": "LAB-RUN-001",
                "matrix_id": "MATRIX-GOVERNOR",
                "target_id": "local-target",
                "dry_run": false,
                "trials": [
                    {
                        "trial_id": "TRIAL-001",
                        "factors": { "governor": "performance" },
                        "status": "blocked",
                        "artifact_refs": [],
                        "failure": "controlled factor 'governor' is not supported by PR6 real runner",
                        "started_at_unix_ms": 1,
                        "ended_at_unix_ms": 2
                    }
                ],
                "time_unix_ms": 3
            })
            .to_string(),
        )
        .unwrap();

        let coverage = operating_point_coverage(&run_dir, "local-target".to_string()).unwrap();
        assert_eq!(
            coverage.coverage_status,
            OperatingPointCoverageStatus::NotControllable
        );
        assert!(coverage
            .blocked_points
            .iter()
            .any(|point| point.factor_id == "governor"
                && point.coverage_status == OperatingPointCoverageStatus::NotControllable));
    }

    fn target_inventory_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": "lab.target_inventory.v1",
            "target_id": "pi4-target55",
            "target": "ssh://pi4-demo",
            "collected_by": "adc-lab",
            "time_unix_ms": 1780000000000u64,
            "software_stack": {
                "os": "linux",
                "kernel": "6.x",
                "arch": "aarch64",
                "board": "raspberry_pi_4"
            },
            "hardware": {
                "cpu_count": 4,
                "memory_total_kb": 1024,
                "thermal_zones": 1,
                "cpufreq_policies": 1
            },
            "control_surfaces": []
        })
    }

    fn create_completed_core_artifacts(run_dir: &Path) {
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::create_dir_all(run_dir.join("toolchain")).unwrap();
        fs::create_dir_all(run_dir.join("observations")).unwrap();
        fs::create_dir_all(run_dir.join("tools")).unwrap();
        fs::write(
            run_dir.join("inventory/target_inventory.json"),
            target_inventory_json().to_string(),
        )
        .unwrap();
        fs::write(run_dir.join("toolchain/toolchain_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("observations/observe.json"), "{}").unwrap();
        fs::write(run_dir.join("tools/tool_qualification_summary.json"), "{}").unwrap();
        fs::write(
            run_dir.join("tools/adc-lab-target.version.json"),
            serde_json::to_string(&test_build_info("adc-lab-target")).unwrap(),
        )
        .unwrap();
    }

    fn audit_line(run_id: &str, operation: &str) -> String {
        serde_json::json!({
            "schema_version": "lab.audit_event.v1",
            "event_id": "EVT-001",
            "run_id": run_id,
            "target_id": "local-target",
            "actor": { "kind": "agent", "id": "codex" },
            "operation": operation,
            "operation_id": null,
            "risk_tier": "tier0_read_only_observation",
            "approval_ref": null,
            "restore_lease_ref": null,
            "result": "recorded",
            "policy_version": "default-lab-policy-v1",
            "time_unix_ms": 1
        })
        .to_string()
    }

    fn test_build_info(name: &str) -> BuildInfo {
        BuildInfo {
            name: name.to_string(),
            version: "0.1.10".to_string(),
            git_sha: "test-git-sha".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            build_profile: "test".to_string(),
        }
    }
}
