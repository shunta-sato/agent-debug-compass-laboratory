use crate::contracts::{
    ArchitectureOptionEvidence, CapabilityClaimBoundary, CapabilityClass, CapabilityCostDimension,
    CapabilityCostModel, CapabilityCostModelStatus, CapabilityEvidence, CapabilityEvidenceStatus,
    ClaimDecision, ClaimEvidenceTrace, ClaimTraceEntry, CostDimensionKind, CostEvidenceStatus,
    ExperimentRun, ExperimentTrial, FamiliarizationPack, OperatingPointBlockedPoint,
    OperatingPointClaimBoundary, OperatingPointCoverage, OperatingPointCoveragePoint,
    OperatingPointCoverageStatus, OperatingPointEvidenceClass, RunArtifactRef, RunDataQuality,
    RunManifest, TargetInventory,
};
use crate::ids::now_unix_ms;
use crate::{artifact_uri_for_run, LabResult};
use serde::de::DeserializeOwned;
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

pub fn capability_cost_model(
    run_dir: impl AsRef<Path>,
    target_id: String,
) -> LabResult<CapabilityCostModel> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let target_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let toolchain_ref =
        artifact_ref_if_exists(run_dir, &run_id, "toolchain/toolchain_inventory.json")?;
    let coverage_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/operating_point_coverage.json")?;
    let target_inventory: Option<TargetInventory> =
        read_json_artifact_if_exists(run_dir.join("inventory/target_inventory.json"))?;
    let load_refs = load_result_artifact_refs(run_dir, &run_id)?;

    let mut evidence_refs = Vec::new();
    evidence_refs.extend(target_inventory_ref.clone());
    evidence_refs.extend(toolchain_ref.clone());
    evidence_refs.extend(coverage_ref.clone());
    evidence_refs.extend(load_refs.clone());
    evidence_refs.sort();
    evidence_refs.dedup();

    let model_status =
        capability_model_status(&target_id, target_inventory_ref.as_ref(), &load_refs);
    let capabilities = capability_evidence(
        target_inventory.as_ref(),
        target_inventory_ref.as_ref(),
        toolchain_ref.as_ref(),
        coverage_ref.as_ref(),
        &load_refs,
    );
    let architecture_options = architecture_options(
        target_inventory_ref.as_ref(),
        coverage_ref.as_ref(),
        &load_refs,
    );
    let blocked_claims = capability_blocked_claims(coverage_ref.as_ref(), &load_refs);
    let limitations = capability_limitations(&target_id, target_inventory_ref.as_ref(), &load_refs);

    Ok(CapabilityCostModel {
        schema_version: "lab.capability_cost_model.v1".to_string(),
        run_id,
        target_id,
        model_status,
        capabilities,
        architecture_options,
        blocked_claims,
        limitations,
        evidence_refs,
        time_unix_ms: now_unix_ms(),
    })
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

fn capability_model_status(
    target_id: &str,
    target_inventory_ref: Option<&String>,
    load_refs: &[String],
) -> CapabilityCostModelStatus {
    if target_inventory_ref.is_none() && load_refs.is_empty() {
        CapabilityCostModelStatus::InsufficientEvidence
    } else if target_id == "local-target" {
        CapabilityCostModelStatus::HostFallbackOnly
    } else {
        CapabilityCostModelStatus::TargetEvidencePartial
    }
}

fn capability_evidence(
    inventory: Option<&TargetInventory>,
    inventory_ref: Option<&String>,
    toolchain_ref: Option<&String>,
    coverage_ref: Option<&String>,
    load_refs: &[String],
) -> Vec<CapabilityEvidence> {
    let mut capabilities = Vec::new();
    if let Some(inventory) = inventory {
        capabilities.push(cpu_capability(inventory, inventory_ref));
        capabilities.push(memory_capability(inventory, inventory_ref));
        capabilities.push(thermal_capability(inventory, inventory_ref));
        capabilities.push(cpufreq_capability(inventory, inventory_ref, coverage_ref));
    } else {
        capabilities.push(missing_capability(
            "cpu_topology",
            CapabilityClass::Cpu,
            "CPU topology was not observed in this run",
            "Run target inventory before making CPU architecture claims.",
        ));
        capabilities.push(missing_capability(
            "memory_capacity",
            CapabilityClass::Memory,
            "Memory capacity was not observed in this run",
            "Run target inventory before making memory budget claims.",
        ));
    }

    capabilities.push(load_response_capability(load_refs));
    capabilities.push(missing_capability(
        "gpu_acceleration",
        CapabilityClass::Gpu,
        "GPU capability and workload cost were not qualified in this run",
        "GPU presence alone is not evidence that GPU offload is better.",
    ));
    capabilities.push(missing_capability(
        "npu_acceleration",
        CapabilityClass::Npu,
        "NPU capability and workload cost were not qualified in this run",
        "NPU presence alone is not evidence that NPU offload is better.",
    ));
    capabilities.push(missing_capability(
        "dsp_acceleration",
        CapabilityClass::Dsp,
        "DSP capability and workload cost were not qualified in this run",
        "DSP presence alone is not evidence that DSP offload is better.",
    ));
    capabilities.push(storage_capability(toolchain_ref));
    capabilities.push(network_capability(toolchain_ref));
    capabilities
}

fn cpu_capability(
    inventory: &TargetInventory,
    inventory_ref: Option<&String>,
) -> CapabilityEvidence {
    let evidence_refs = optional_ref_vec(inventory_ref);
    CapabilityEvidence {
        capability_id: "cpu_topology".to_string(),
        capability_class: CapabilityClass::Cpu,
        status: CapabilityEvidenceStatus::Observed,
        summary: format!("{} CPU worker(s) observed", inventory.hardware.cpu_count),
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension: CostDimensionKind::Cpu,
            status: CostEvidenceStatus::Provisional,
            summary: "CPU topology is observed; sustained CPU cost is not calibrated".to_string(),
            evidence_refs,
            limitation: "CPU availability does not prove production headroom or thermal safety"
                .to_string(),
        }],
        claim_boundary:
            "CPU availability can support bounded lab baseline experiments, not production readiness."
                .to_string(),
    }
}

fn memory_capability(
    inventory: &TargetInventory,
    inventory_ref: Option<&String>,
) -> CapabilityEvidence {
    let evidence_refs = optional_ref_vec(inventory_ref);
    let (status, summary) = if let Some(memory_total_kb) = inventory.hardware.memory_total_kb {
        (
            CapabilityEvidenceStatus::Observed,
            format!("{memory_total_kb} KiB total memory observed"),
        )
    } else {
        (
            CapabilityEvidenceStatus::NeedsProbe,
            "memory total was not available from inventory".to_string(),
        )
    };
    CapabilityEvidence {
        capability_id: "memory_capacity".to_string(),
        capability_class: CapabilityClass::Memory,
        status,
        summary,
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension: CostDimensionKind::Memory,
            status: CostEvidenceStatus::Provisional,
            summary: "Memory capacity may be observed, but RSS/heap growth is not calibrated"
                .to_string(),
            evidence_refs,
            limitation: "Memory capacity does not prove memory pressure behavior".to_string(),
        }],
        claim_boundary: "Memory capacity is inventory evidence, not workload memory cost evidence."
            .to_string(),
    }
}

fn thermal_capability(
    inventory: &TargetInventory,
    inventory_ref: Option<&String>,
) -> CapabilityEvidence {
    let evidence_refs = optional_ref_vec(inventory_ref);
    CapabilityEvidence {
        capability_id: "thermal_surface".to_string(),
        capability_class: CapabilityClass::Thermal,
        status: if inventory.hardware.thermal_zones > 0 {
            CapabilityEvidenceStatus::Observed
        } else {
            CapabilityEvidenceStatus::MissingEvidence
        },
        summary: format!("{} thermal zone(s) observed", inventory.hardware.thermal_zones),
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension: CostDimensionKind::Thermal,
            status: CostEvidenceStatus::Provisional,
            summary: "Thermal surface availability is observed; sustained thermal behavior is not calibrated"
                .to_string(),
            evidence_refs,
            limitation: "Thermal sensor presence does not prove thermally safe operation".to_string(),
        }],
        claim_boundary:
            "Thermal surface availability enables monitoring, not thermal safety claims."
                .to_string(),
    }
}

fn cpufreq_capability(
    inventory: &TargetInventory,
    inventory_ref: Option<&String>,
    coverage_ref: Option<&String>,
) -> CapabilityEvidence {
    let mut evidence_refs = optional_ref_vec(inventory_ref);
    evidence_refs.extend(optional_ref_vec(coverage_ref));
    CapabilityEvidence {
        capability_id: "cpu_frequency_surface".to_string(),
        capability_class: CapabilityClass::FrequencyControl,
        status: if inventory.hardware.cpufreq_policies > 0 {
            CapabilityEvidenceStatus::Observed
        } else {
            CapabilityEvidenceStatus::MissingEvidence
        },
        summary: format!(
            "{} cpufreq policy surface(s) observed",
            inventory.hardware.cpufreq_policies
        ),
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension: CostDimensionKind::Cpu,
            status: CostEvidenceStatus::Provisional,
            summary: "Frequency surface exists, but fixed-frequency cost is not measured"
                .to_string(),
            evidence_refs,
            limitation: "Observed dynamic frequency variation is not a controlled sweep"
                .to_string(),
        }],
        claim_boundary:
            "cpufreq surface presence does not prove behavior across fixed operating points."
                .to_string(),
    }
}

fn load_response_capability(load_refs: &[String]) -> CapabilityEvidence {
    let has_load = !load_refs.is_empty();
    CapabilityEvidence {
        capability_id: "bounded_cpu_load_response".to_string(),
        capability_class: CapabilityClass::Load,
        status: if has_load {
            CapabilityEvidenceStatus::Observed
        } else {
            CapabilityEvidenceStatus::MissingEvidence
        },
        summary: if has_load {
            format!("{} bounded load result artifact(s) found", load_refs.len())
        } else {
            "No bounded load result artifacts found".to_string()
        },
        evidence_refs: load_refs.to_vec(),
        cost_dimensions: vec![
            CapabilityCostDimension {
                dimension: CostDimensionKind::Cpu,
                status: if has_load {
                    CostEvidenceStatus::MeasuredPartial
                } else {
                    CostEvidenceStatus::MissingEvidence
                },
                summary: "Bounded CPU load response is lab evidence only".to_string(),
                evidence_refs: load_refs.to_vec(),
                limitation: "Short bounded load does not prove sustained CPU budget".to_string(),
            },
            CapabilityCostDimension {
                dimension: CostDimensionKind::Thermal,
                status: if has_load {
                    CostEvidenceStatus::MeasuredPartial
                } else {
                    CostEvidenceStatus::MissingEvidence
                },
                summary: "Load results may include thermal monitor output when surface is available"
                    .to_string(),
                evidence_refs: load_refs.to_vec(),
                limitation: "Bounded load result does not prove sustained thermal safety".to_string(),
            },
        ],
        claim_boundary:
            "Bounded load response can support lab workload claims, not production physical-footprint claims."
                .to_string(),
    }
}

fn storage_capability(toolchain_ref: Option<&String>) -> CapabilityEvidence {
    let evidence_refs = optional_ref_vec(toolchain_ref);
    CapabilityEvidence {
        capability_id: "storage_io".to_string(),
        capability_class: CapabilityClass::Storage,
        status: CapabilityEvidenceStatus::MissingEvidence,
        summary: "Storage throughput, writes, and flash wear were not measured".to_string(),
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![
            CapabilityCostDimension {
                dimension: CostDimensionKind::StorageWrites,
                status: CostEvidenceStatus::MissingEvidence,
                summary: "No storage write budget evidence".to_string(),
                evidence_refs: evidence_refs.clone(),
                limitation: "Storage-heavy architecture claims require write and fsync evidence"
                    .to_string(),
            },
            CapabilityCostDimension {
                dimension: CostDimensionKind::FlashWear,
                status: CostEvidenceStatus::MissingEvidence,
                summary: "No flash wear estimate".to_string(),
                evidence_refs,
                limitation: "Flash-safe claims require media and write-rate evidence".to_string(),
            },
        ],
        claim_boundary: "Storage capability is unqualified for architecture decisions in this run."
            .to_string(),
    }
}

fn network_capability(toolchain_ref: Option<&String>) -> CapabilityEvidence {
    let evidence_refs = optional_ref_vec(toolchain_ref);
    CapabilityEvidence {
        capability_id: "network_io".to_string(),
        capability_class: CapabilityClass::Network,
        status: CapabilityEvidenceStatus::MissingEvidence,
        summary: "Network throughput, latency, and radio/power cost were not measured".to_string(),
        evidence_refs: evidence_refs.clone(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension: CostDimensionKind::Network,
            status: CostEvidenceStatus::MissingEvidence,
            summary: "No network cost evidence".to_string(),
            evidence_refs,
            limitation:
                "Network-heavy architecture claims require target-specific network evidence"
                    .to_string(),
        }],
        claim_boundary: "Network capability is unqualified for architecture decisions in this run."
            .to_string(),
    }
}

fn missing_capability(
    capability_id: &str,
    capability_class: CapabilityClass,
    summary: &str,
    claim_boundary: &str,
) -> CapabilityEvidence {
    let dimension = missing_cost_dimension_for_capability(&capability_class);
    CapabilityEvidence {
        capability_id: capability_id.to_string(),
        capability_class,
        status: CapabilityEvidenceStatus::MissingEvidence,
        summary: summary.to_string(),
        evidence_refs: Vec::new(),
        cost_dimensions: vec![CapabilityCostDimension {
            dimension,
            status: CostEvidenceStatus::MissingEvidence,
            summary: "No cost evidence collected for this capability".to_string(),
            evidence_refs: Vec::new(),
            limitation: "Architecture claims require capability-specific measurement".to_string(),
        }],
        claim_boundary: claim_boundary.to_string(),
    }
}

fn missing_cost_dimension_for_capability(capability_class: &CapabilityClass) -> CostDimensionKind {
    match capability_class {
        CapabilityClass::Cpu => CostDimensionKind::Cpu,
        CapabilityClass::Memory => CostDimensionKind::Memory,
        CapabilityClass::Thermal => CostDimensionKind::Thermal,
        CapabilityClass::FrequencyControl | CapabilityClass::Load => CostDimensionKind::Cpu,
        CapabilityClass::Gpu | CapabilityClass::Npu | CapabilityClass::Dsp => {
            CostDimensionKind::LatencyJitter
        }
        CapabilityClass::Storage => CostDimensionKind::StorageWrites,
        CapabilityClass::Network => CostDimensionKind::Network,
    }
}

fn architecture_options(
    target_inventory_ref: Option<&String>,
    coverage_ref: Option<&String>,
    load_refs: &[String],
) -> Vec<ArchitectureOptionEvidence> {
    let mut cpu_refs = optional_ref_vec(target_inventory_ref);
    cpu_refs.extend(load_refs.to_vec());
    vec![
        ArchitectureOptionEvidence {
            option_id: "cpu_baseline".to_string(),
            summary: "Use CPU as the baseline implementation path for lab experiments".to_string(),
            decision: if target_inventory_ref.is_some() {
                ClaimDecision::Supported
            } else {
                ClaimDecision::Blocked
            },
            rationale: "CPU inventory is the minimum supported architecture evidence; bounded load evidence strengthens lab-only claims."
                .to_string(),
            evidence_refs: cpu_refs,
            next_evidence_needed: vec![
                "production CPU budget calibration".to_string(),
                "sustained thermal and observer-effect evidence".to_string(),
            ],
        },
        ArchitectureOptionEvidence {
            option_id: "gpu_offload".to_string(),
            summary: "Move workload to GPU".to_string(),
            decision: ClaimDecision::Blocked,
            rationale: "No GPU capability, workload fit, transfer cost, thermal cost, or tool qualification evidence exists in this run."
                .to_string(),
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "GPU capability discovery".to_string(),
                "GPU tool qualification".to_string(),
                "CPU-vs-GPU workload cost comparison".to_string(),
            ],
        },
        ArchitectureOptionEvidence {
            option_id: "accelerator_offload".to_string(),
            summary: "Move workload to NPU/DSP/other accelerator".to_string(),
            decision: ClaimDecision::Blocked,
            rationale: "No NPU/DSP capability or qualified workload-cost evidence exists in this run."
                .to_string(),
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "accelerator capability discovery".to_string(),
                "adapter/tool qualification".to_string(),
                "controlled cost comparison".to_string(),
            ],
        },
        ArchitectureOptionEvidence {
            option_id: "fixed_frequency_optimization".to_string(),
            summary: "Optimize architecture using fixed CPU frequency assumptions".to_string(),
            decision: ClaimDecision::Blocked,
            rationale: "Operating-point coverage does not include controlled fixed-frequency evidence."
                .to_string(),
            evidence_refs: optional_ref_vec(coverage_ref),
            next_evidence_needed: vec![
                "approved privileged frequency control".to_string(),
                "controlled fixed-frequency matrix".to_string(),
                "restore verification per point".to_string(),
            ],
        },
        ArchitectureOptionEvidence {
            option_id: "storage_heavy_pipeline".to_string(),
            summary: "Use storage-heavy buffering or logging architecture".to_string(),
            decision: ClaimDecision::Blocked,
            rationale: "No storage write, flash wear, or degraded storage-pressure evidence exists."
                .to_string(),
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "bounded storage write experiment".to_string(),
                "flash wear budget".to_string(),
                "storage pressure recovery evidence".to_string(),
            ],
        },
    ]
}

fn capability_blocked_claims(
    coverage_ref: Option<&String>,
    load_refs: &[String],
) -> Vec<CapabilityClaimBoundary> {
    vec![
        CapabilityClaimBoundary {
            claim: "GPU presence means GPU offload is better".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "qualified GPU capability evidence".to_string(),
                "workload-specific CPU/GPU cost comparison".to_string(),
            ],
        },
        CapabilityClaimBoundary {
            claim: "NPU/DSP offload is supported by this run".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: Vec::new(),
            next_evidence_needed: vec![
                "accelerator discovery".to_string(),
                "qualified adapter output".to_string(),
                "controlled cost comparison".to_string(),
            ],
        },
        CapabilityClaimBoundary {
            claim: "bounded CPU load proves production readiness".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: load_refs.to_vec(),
            next_evidence_needed: vec![
                "production resource budgets".to_string(),
                "sustained thermal evidence".to_string(),
                "wakeups, battery, flash, and jitter evidence".to_string(),
            ],
        },
        CapabilityClaimBoundary {
            claim: "observed dynamic CPU frequency range is a fixed-frequency sweep".to_string(),
            decision: ClaimDecision::Blocked,
            evidence_refs: optional_ref_vec(coverage_ref),
            next_evidence_needed: vec![
                "controlled fixed-frequency operating-point matrix".to_string(),
                "approved privileged control and restore evidence".to_string(),
            ],
        },
    ]
}

fn capability_limitations(
    target_id: &str,
    target_inventory_ref: Option<&String>,
    load_refs: &[String],
) -> Vec<String> {
    let mut limitations = Vec::new();
    if target_inventory_ref.is_none() {
        limitations.push("target inventory is missing".to_string());
    }
    if target_id == "local-target" {
        limitations.push("local host fallback cannot prove Pi4/Pi5 physical footprint".to_string());
    }
    if load_refs.is_empty() {
        limitations.push("bounded load cost evidence is missing".to_string());
    }
    limitations.extend([
        "GPU/NPU/DSP capability and workload fit are not qualified".to_string(),
        "storage, flash wear, wakeup, battery, latency, and jitter costs are not measured"
            .to_string(),
        "capability presence does not imply architecture suitability".to_string(),
    ]);
    limitations
}

fn optional_ref_vec(reference: Option<&String>) -> Vec<String> {
    reference.cloned().into_iter().collect()
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

    #[test]
    fn contract_validation_capability_cost_model_uses_inventory_evidence_and_blocks_offload_claims()
    {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::create_dir_all(run_dir.join("toolchain")).unwrap();
        fs::create_dir_all(run_dir.join("reports")).unwrap();
        fs::write(
            run_dir.join("inventory/target_inventory.json"),
            target_inventory_json().to_string(),
        )
        .unwrap();
        fs::write(run_dir.join("toolchain/toolchain_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("reports/operating_point_coverage.json"), "{}").unwrap();

        let model = capability_cost_model(&run_dir, "pi4-target55".to_string()).unwrap();
        assert_eq!(
            model.model_status,
            CapabilityCostModelStatus::TargetEvidencePartial
        );
        assert!(model
            .evidence_refs
            .iter()
            .all(|artifact| artifact.starts_with("artifact://lab/runs/LAB-RUN-001/")));

        let cpu = capability(&model, "cpu_topology");
        assert_eq!(cpu.status, CapabilityEvidenceStatus::Observed);
        assert_eq!(cpu.cost_dimensions[0].dimension, CostDimensionKind::Cpu);
        let memory = capability(&model, "memory_capacity");
        assert_eq!(memory.status, CapabilityEvidenceStatus::Observed);
        let thermal = capability(&model, "thermal_surface");
        assert_eq!(thermal.status, CapabilityEvidenceStatus::Observed);
        let cpufreq = capability(&model, "cpu_frequency_surface");
        assert_eq!(cpufreq.status, CapabilityEvidenceStatus::Observed);

        let gpu = capability(&model, "gpu_acceleration");
        assert_eq!(gpu.status, CapabilityEvidenceStatus::MissingEvidence);
        assert_eq!(
            gpu.cost_dimensions[0].dimension,
            CostDimensionKind::LatencyJitter
        );
        assert!(model
            .architecture_options
            .iter()
            .any(|option| option.option_id == "gpu_offload"
                && option.decision == ClaimDecision::Blocked));
        assert!(model.blocked_claims.iter().any(|claim| {
            claim.decision == ClaimDecision::Blocked && claim.claim.contains("GPU presence")
        }));
        assert!(model.blocked_claims.iter().any(|claim| {
            claim.decision == ClaimDecision::Blocked
                && claim.claim.contains("fixed-frequency sweep")
        }));
    }

    #[test]
    fn contract_validation_capability_cost_model_records_bounded_load_as_partial_lab_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("experiments/trials/TRIAL-001")).unwrap();
        fs::write(
            run_dir.join("experiments/trials/TRIAL-001/load_result.json"),
            "{}",
        )
        .unwrap();

        let model = capability_cost_model(&run_dir, "local-target".to_string()).unwrap();
        assert_eq!(
            model.model_status,
            CapabilityCostModelStatus::HostFallbackOnly
        );
        let load = capability(&model, "bounded_cpu_load_response");
        assert_eq!(load.status, CapabilityEvidenceStatus::Observed);
        assert!(load
            .evidence_refs
            .iter()
            .any(|artifact| artifact.ends_with("/load_result.json")));
        assert!(load
            .cost_dimensions
            .iter()
            .any(|dimension| dimension.dimension == CostDimensionKind::Cpu
                && dimension.status == CostEvidenceStatus::MeasuredPartial));
        assert!(model.blocked_claims.iter().any(|claim| {
            claim.claim == "bounded CPU load proves production readiness"
                && claim.decision == ClaimDecision::Blocked
                && claim
                    .evidence_refs
                    .iter()
                    .any(|artifact| artifact.ends_with("/load_result.json"))
        }));
    }

    #[test]
    fn contract_validation_capability_cost_model_rejects_malformed_inventory_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::write(run_dir.join("inventory/target_inventory.json"), "{bad").unwrap();

        let error = capability_cost_model(&run_dir, "pi4-target55".to_string()).unwrap_err();
        assert!(error.to_string().contains("failed to parse JSON artifact"));
    }

    fn capability<'a>(
        model: &'a CapabilityCostModel,
        capability_id: &str,
    ) -> &'a CapabilityEvidence {
        model
            .capabilities
            .iter()
            .find(|capability| capability.capability_id == capability_id)
            .unwrap_or_else(|| panic!("missing capability {capability_id}"))
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
}
