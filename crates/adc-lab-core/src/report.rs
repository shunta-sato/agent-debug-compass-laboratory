use crate::contracts::{
    CapabilityCostModel, ClaimDecision, ClaimEvidenceTrace, ClaimTraceEntry, ExperimentRun,
    ExperimentTrial, FamiliarizationPack, OperatingPointBlockedPoint, OperatingPointClaimBoundary,
    OperatingPointCoverage, OperatingPointCoveragePoint, OperatingPointCoverageStatus,
    OperatingPointEvidenceClass, RunArtifactRef, RunDataQuality, RunManifest,
};
use crate::ids::now_unix_ms;
use crate::{artifact_uri_for_run, LabResult};
use std::collections::BTreeSet;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

const READ_ONLY_REQUIRED_AUDIT_OPS: &[&str] = &[
    "inventory",
    "toolchain.discover",
    "tool.qualify_inventory",
    "observe",
];

pub fn pack_run(run_dir: impl AsRef<Path>, target_id: String) -> LabResult<FamiliarizationPack> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let target_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let toolchain_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "toolchain/toolchain_inventory.json")?;
    let tool_qualification_summary_ref =
        artifact_ref_if_exists(run_dir, &run_id, "tools/tool_qualification_summary.json")?;
    let observation_ref = artifact_ref_if_exists(run_dir, &run_id, "observations/observe.json")?;
    let artifact_refs = collect_artifact_refs(run_dir, &run_id)?;
    let audit_event_count = count_audit_events(run_dir.join("audit.jsonl"))?;
    let restore_status = restore_status(run_dir)?;
    let claim_trace_ref = artifact_refs
        .iter()
        .find(|path| path.ends_with("claim_evidence_trace.json"))
        .cloned();
    let has_read_only_core = target_inventory_ref.is_some()
        && toolchain_inventory_ref.is_some()
        && observation_ref.is_some();
    Ok(FamiliarizationPack {
        schema_version: "lab.familiarization_pack.v1".to_string(),
        run_id,
        target_id,
        pack_status: if has_read_only_core {
            "observational_read_only".to_string()
        } else {
            "incomplete".to_string()
        },
        artifact_refs,
        supported_claims: supported_read_only_claims(
            target_inventory_ref.as_ref(),
            toolchain_inventory_ref.as_ref(),
            tool_qualification_summary_ref.as_ref(),
            observation_ref.as_ref(),
        ),
        blocked_claims: blocked_read_only_claims(),
        next_evidence_needed: next_read_only_evidence_needed(),
        audit_event_count,
        restore_status,
        claim_trace_ref,
        time_unix_ms: now_unix_ms(),
    })
}

pub fn read_only_claim_trace(
    run_dir: impl AsRef<Path>,
    target_id: String,
) -> LabResult<ClaimEvidenceTrace> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let target_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let toolchain_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "toolchain/toolchain_inventory.json")?;
    let tool_qualification_summary_ref =
        artifact_ref_if_exists(run_dir, &run_id, "tools/tool_qualification_summary.json")?;
    let observation_ref = artifact_ref_if_exists(run_dir, &run_id, "observations/observe.json")?;

    Ok(ClaimEvidenceTrace {
        schema_version: "lab.claim_evidence_trace.v1".to_string(),
        run_id,
        target_id,
        claims: vec![
            claim_entry(
                "target identity was observed through read-only inventory surfaces",
                target_inventory_ref,
                "read-only target inventory",
            ),
            claim_entry(
                "toolchain availability was observed through read-only discovery",
                toolchain_inventory_ref,
                "read-only toolchain inventory",
            ),
            claim_entry(
                "tool qualification summary was generated for discovered toolchain",
                tool_qualification_summary_ref,
                "tool qualification summary",
            ),
            ClaimTraceEntry {
                claim: "passive resource signals were sampled under the current target policy"
                    .to_string(),
                decision: if observation_ref.is_some() {
                    ClaimDecision::Provisional
                } else {
                    ClaimDecision::Blocked
                },
                evidence_refs: observation_ref.into_iter().collect(),
                next_evidence_needed: vec![
                    "controlled operating point matrix".to_string(),
                    "observer effect calibration".to_string(),
                ],
            },
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
                claim: "target runtime is production-ready, battery-safe, flash-safe, low-overhead, and thermally safe".to_string(),
                decision: ClaimDecision::Blocked,
                evidence_refs: vec![],
                next_evidence_needed: vec![
                    "target-specific resource budgets".to_string(),
                    "bounded load and safety monitor evidence".to_string(),
                    "sustained thermal and observer-effect evidence".to_string(),
                ],
            },
        ],
        time_unix_ms: now_unix_ms(),
    })
}

pub fn run_manifest(
    run_dir: impl AsRef<Path>,
    target_id: String,
    target: String,
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
    adc_lab_version: String,
) -> LabResult<RunManifest> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let artifacts = known_run_artifacts(run_dir, &run_id)?;
    let audit_ref = artifact_ref_if_exists(run_dir, &run_id, "audit.jsonl")?
        .unwrap_or_else(|| format!("artifact://lab/runs/{run_id}/audit.jsonl"));
    let claim_trace_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/claim_evidence_trace.json")?;
    let missing = read_only_data_quality_missing(run_dir, &run_id)?;
    Ok(RunManifest {
        schema_version: "lab.run_manifest.v1".to_string(),
        run_id,
        target_id,
        target,
        mode: "read_only_familiarization".to_string(),
        started_at_unix_ms,
        ended_at_unix_ms,
        adc_lab_version,
        artifacts,
        audit_ref,
        claim_trace_ref,
        data_quality: RunDataQuality { missing },
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
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let experiment = serde_json::from_slice(&bytes).map_err(|error| {
        crate::LabError::Validation(format!(
            "failed to parse experiment run artifact {}: {error}",
            path.display()
        ))
    })?;
    Ok(Some(experiment))
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

pub fn capability_cost_model(run_id: String, target_id: String) -> CapabilityCostModel {
    let (cost_model_status, limitations) = if target_id == "local-target" {
        (
            "provisional_host_fallback_only".to_string(),
            vec![
                "host fallback cannot prove Pi4/Pi5 physical footprint".to_string(),
                "target characterization is required before production budget claims".to_string(),
            ],
        )
    } else {
        (
            "provisional_short_target_smoke".to_string(),
            vec![
                "short bounded target smoke does not calibrate production NFR budgets".to_string(),
                "longer operating-envelope and degraded-mode runs are required before production budget claims".to_string(),
            ],
        )
    };
    CapabilityCostModel {
        schema_version: "lab.capability_cost_model.v1".to_string(),
        run_id,
        target_id,
        capabilities: vec!["cpu_load_response".to_string(), "thermal_trend".to_string()],
        cost_model_status,
        limitations,
        time_unix_ms: now_unix_ms(),
    }
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
    run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("LAB-RUN-unknown")
        .to_string()
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

fn read_only_data_quality_missing(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    let mut missing = Vec::new();
    for (label, relative_path) in [
        (
            "target inventory artifact missing",
            "inventory/target_inventory.json",
        ),
        (
            "toolchain inventory artifact missing",
            "toolchain/toolchain_inventory.json",
        ),
        (
            "passive observation artifact missing",
            "observations/observe.json",
        ),
        (
            "tool qualification summary artifact missing",
            "tools/tool_qualification_summary.json",
        ),
    ] {
        if artifact_ref_if_exists(run_dir, run_id, relative_path)?.is_none() {
            missing.push(label.to_string());
        }
    }

    let operations = audit_operations(run_dir.join("audit.jsonl"))?;
    for required in READ_ONLY_REQUIRED_AUDIT_OPS {
        if !operations.iter().any(|operation| operation == required) {
            missing.push(format!("audit event missing for {required}"));
        }
    }

    missing.push("no controlled operating point experiment was run".to_string());
    missing.push("no privileged control operation was run".to_string());
    missing.push("no load or stress experiment was run".to_string());
    Ok(missing)
}

fn audit_operations(path: PathBuf) -> LabResult<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let mut operations = Vec::new();
    for line in std::io::BufReader::new(file).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if let Some(operation) = value.get("operation").and_then(|value| value.as_str()) {
            operations.push(operation.to_string());
        }
    }
    Ok(operations)
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

fn supported_read_only_claims(
    target_inventory_ref: Option<&String>,
    toolchain_inventory_ref: Option<&String>,
    tool_qualification_summary_ref: Option<&String>,
    observation_ref: Option<&String>,
) -> Vec<String> {
    let mut claims = Vec::new();
    if target_inventory_ref.is_some() {
        claims.push("target inventory was collected through read-only surfaces".to_string());
    }
    if toolchain_inventory_ref.is_some() {
        claims.push("toolchain availability was collected through read-only discovery".to_string());
    }
    if tool_qualification_summary_ref.is_some() {
        claims.push("tool qualification summary was generated".to_string());
    }
    if observation_ref.is_some() {
        claims.push("observed covariates were sampled under the current target policy".to_string());
    }
    claims
}

fn blocked_read_only_claims() -> Vec<String> {
    vec![
        "low overhead across all operating points".to_string(),
        "battery safe".to_string(),
        "fixed CPU frequency behavior".to_string(),
        "production readiness".to_string(),
        "thermal safety under load".to_string(),
        "no observer effect".to_string(),
    ]
}

fn next_read_only_evidence_needed() -> Vec<String> {
    vec![
        "controlled operating point matrix".to_string(),
        "tool qualification for privileged control".to_string(),
        "bounded load with safety monitor".to_string(),
        "target-specific resource budget calibration".to_string(),
    ]
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

fn count_audit_events(path: PathBuf) -> LabResult<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let file = fs::File::open(path)?;
    Ok(std::io::BufReader::new(file).lines().count())
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
            "0.1.0".to_string(),
        )
        .unwrap();
        assert!(manifest
            .artifacts
            .iter()
            .all(|artifact| artifact.artifact_ref.starts_with("artifact://lab/runs/")));
        assert!(manifest
            .data_quality
            .missing
            .contains(&"no controlled operating point experiment was run".to_string()));
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
}
