use crate::contracts::{ExperimentRun, ExperimentTrial};
use crate::evidence::{
    claim, claim_definition, Artifact, Claim, DataQuality, DataQualityLevel, Decision,
    EvidenceStore, Kind, Status,
};
use crate::ids::{new_id, now_unix_ms};
use crate::report::{
    artifact_ref_if_exists, collect_artifact_refs, experiment_has_completed_trial, pack_status,
    read_json_artifact_if_exists, restore_status, run_evidence_summary, RunEvidenceSummary,
    OP_BOUNDED_LOAD, OP_CONTROLLED_OPERATING_POINT, STATUS_COMPLETED,
};
use crate::rules::engine::RuleEvaluation;
use crate::run_validation::{GovernorValidity, RunValidationPayload};
use crate::workflow_profile::supported_validation_profile;
use crate::LabResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const RUN_REPORT_RULE_SET_ID: &str = "rules.run_report.v2";
pub const RUN_REPORT_RELATIVE_PATH: &str = "reports/run_report.v2.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunReportPayload {
    pub rule_set_id: String,
    pub report_status: String,
    pub operations_summary: BTreeMap<String, String>,
    pub operation_audit_refs: BTreeMap<String, String>,
    pub artifact_refs: Vec<String>,
    pub indexed_v2_artifact_refs: Vec<String>,
    pub audit_event_count: usize,
    pub restore_status: String,
    pub operating_point: RunOperatingPointSummary,
    pub evaluations: Vec<RuleEvaluation>,
    pub blocked_claims: Vec<String>,
    pub next_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunOperatingPointSummary {
    pub coverage_status: String,
    pub observed_points: Vec<RunOperatingPointObserved>,
    pub controlled_points: Vec<RunOperatingPointControlled>,
    pub blocked_points: Vec<RunOperatingPointBlocked>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunOperatingPointObserved {
    pub factor_id: String,
    pub level: String,
    pub evidence_refs: Vec<String>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunOperatingPointControlled {
    pub factor_id: String,
    pub level: String,
    pub evidence_refs: Vec<String>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunOperatingPointBlocked {
    pub factor_id: String,
    pub requested_level: Option<String>,
    pub coverage_status: String,
    pub reason: String,
    pub next_evidence: Vec<String>,
}

pub fn evaluate_run_report_v2(
    store: &EvidenceStore,
    run_dir: impl AsRef<Path>,
    target_id: impl Into<String>,
) -> LabResult<Artifact<RunReportPayload>> {
    let run_dir = run_dir.as_ref();
    let target_id = target_id.into();
    let summary = run_evidence_summary(run_dir)?;
    let report_status = pack_status(&summary);
    let artifact_refs = collect_artifact_refs(run_dir, &summary.run_id)?;
    let indexed_v2_artifact_refs = indexed_v2_artifact_refs(store);
    let operating_point = operating_point_summary(store, run_dir, &summary)?;
    let evaluations = run_report_evaluations(run_dir, &summary, &operating_point)?;
    let blocked_claims = blocked_claims(&evaluations);
    let next_evidence = next_evidence(&evaluations);
    let status = if report_status == "incomplete" {
        Status::Insufficient
    } else {
        Status::MeasuredPartial
    };
    let mut artifact = Artifact::new(
        Kind::ReportRun,
        new_id("RUN-REPORT"),
        summary.run_id.clone(),
        target_id,
        status,
        RunReportPayload {
            rule_set_id: RUN_REPORT_RULE_SET_ID.to_string(),
            report_status,
            operations_summary: summary.operations_summary.clone(),
            operation_audit_refs: summary.operation_audit_refs.clone(),
            artifact_refs,
            indexed_v2_artifact_refs,
            audit_event_count: summary.audit_event_count,
            restore_status: restore_status(run_dir)?,
            operating_point,
            evaluations,
            blocked_claims,
            next_evidence,
        },
        now_unix_ms(),
    );
    artifact.claims = artifact
        .payload
        .evaluations
        .iter()
        .map(claim_for_run_report_evaluation)
        .collect();
    artifact.evidence_refs = artifact.payload.artifact_refs.clone();
    artifact.data_quality = data_quality_for_summary(&summary);
    Ok(artifact)
}

fn indexed_v2_artifact_refs(store: &EvidenceStore) -> Vec<String> {
    let mut refs = store
        .all()
        .iter()
        .map(|meta| meta.artifact_ref.clone())
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    refs
}

fn data_quality_for_summary(summary: &RunEvidenceSummary) -> DataQuality {
    let mut notes = Vec::new();
    if summary.target_inventory_ref.is_none() {
        notes.push("target inventory artifact missing".to_string());
    }
    if summary.toolchain_inventory_ref.is_none() {
        notes.push("toolchain inventory artifact missing".to_string());
    }
    if summary.tool_qualification_summary_ref.is_none() {
        notes.push("tool qualification summary artifact missing".to_string());
    }
    if summary.observation_ref.is_none() {
        notes.push("passive observation artifact missing".to_string());
    }
    notes.extend(summary.audit_run_id_mismatches.clone());
    DataQuality {
        level: if notes.is_empty() {
            DataQualityLevel::Complete
        } else {
            DataQualityLevel::Partial
        },
        notes,
    }
}

fn operating_point_summary(
    store: &EvidenceStore,
    run_dir: &Path,
    summary: &RunEvidenceSummary,
) -> LabResult<RunOperatingPointSummary> {
    let experiment_ref =
        artifact_ref_if_exists(run_dir, &summary.run_id, "experiments/experiment_run.json")?;
    let experiment_run: Option<ExperimentRun> =
        read_json_artifact_if_exists(run_dir.join("experiments/experiment_run.json"))?;
    let mut observed_points = Vec::new();
    if let Some(artifact_ref) = summary.observation_ref.clone() {
        observed_points.push(RunOperatingPointObserved {
            factor_id: "default_policy_frequency".to_string(),
            level: "observed_current_policy".to_string(),
            evidence_refs: vec![artifact_ref],
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
    add_validation_operating_points(
        store,
        &mut controlled_keys,
        &mut controlled_points,
        &mut blocked_points,
    )?;
    ensure_fixed_frequency_blocked(&mut blocked_points);
    let coverage_status =
        coverage_status(&observed_points, &controlled_points, &blocked_points).to_string();
    Ok(RunOperatingPointSummary {
        coverage_status,
        observed_points,
        controlled_points,
        blocked_points,
    })
}

fn add_completed_trial_points(
    trial: &ExperimentTrial,
    experiment_ref: Option<&String>,
    controlled_keys: &mut BTreeSet<(String, String)>,
    controlled_points: &mut Vec<RunOperatingPointControlled>,
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
        controlled_points.push(RunOperatingPointControlled {
            factor_id: factor_id.clone(),
            level: level.clone(),
            evidence_refs,
            claim_boundary:
                "Workload intensity was controlled for this bounded trial; CPU frequency remains observed under current policy unless privileged frequency-control evidence exists."
                    .to_string(),
        });
    }
}

fn add_blocked_trial_points(
    trial: &ExperimentTrial,
    blocked_points: &mut Vec<RunOperatingPointBlocked>,
) {
    for (factor_id, level) in &trial.factors {
        if factor_id == "cpu_load_workers" && trial.status != "failed" {
            continue;
        }
        blocked_points.push(RunOperatingPointBlocked {
            factor_id: factor_id.clone(),
            requested_level: Some(level.clone()),
            coverage_status: blocked_status_for_factor(factor_id).to_string(),
            reason: trial
                .failure
                .clone()
                .unwrap_or_else(|| format!("trial ended with status {}", trial.status)),
            next_evidence: next_evidence_for_blocked_factor(factor_id),
        });
    }
}

fn add_validation_operating_points(
    store: &EvidenceStore,
    controlled_keys: &mut BTreeSet<(String, String)>,
    controlled_points: &mut Vec<RunOperatingPointControlled>,
    blocked_points: &mut Vec<RunOperatingPointBlocked>,
) -> LabResult<()> {
    let mut measured_governors = BTreeMap::new();
    let mut blocked_governors = BTreeMap::new();
    for meta in store.iter(Kind::ReportRunValidation) {
        let artifact: Artifact<RunValidationPayload> = store.load(meta)?;
        if !supported_validation_profile(&artifact.payload.profile) {
            continue;
        }
        let legacy_identity = !artifact.payload.has_run_set_identity();
        for result in &artifact.payload.governor_results {
            let evidence_refs = validation_evidence_refs(&meta.artifact_ref, result);
            if result.validity == GovernorValidity::Measured && !legacy_identity {
                measured_governors
                    .entry(result.governor.clone())
                    .or_insert_with(Vec::new)
                    .extend(evidence_refs);
            } else {
                blocked_governors
                    .entry(result.governor.clone())
                    .or_insert_with(|| RunOperatingPointBlocked {
                        factor_id: "governor".to_string(),
                        requested_level: Some(result.governor.clone()),
                        coverage_status: validation_coverage_status(result, legacy_identity),
                        reason: validation_reason(result, legacy_identity),
                        next_evidence: validation_next_evidence(result, legacy_identity),
                    });
            }
        }
    }

    for blocked in blocked_governors.values() {
        blocked_points.push(blocked.clone());
    }
    for (governor, mut evidence_refs) in measured_governors {
        if blocked_governors.contains_key(&governor) {
            continue;
        }
        if !controlled_keys.insert(("governor".to_string(), governor.clone())) {
            continue;
        }
        evidence_refs.sort();
        evidence_refs.dedup();
        controlled_points.push(RunOperatingPointControlled {
            factor_id: "governor".to_string(),
            level: governor,
            evidence_refs,
            claim_boundary:
                "Governor control is treated as measured only through report.run_validation evidence linking plan, approval, apply, load, restore, and health-check artifacts."
                    .to_string(),
        });
    }
    Ok(())
}

fn validation_evidence_refs(
    validation_ref: &str,
    result: &crate::run_validation::GovernorValidation,
) -> Vec<String> {
    let mut refs = vec![validation_ref.to_string()];
    refs.extend(
        [
            result.plan_ref.clone(),
            result.approval_ref.clone(),
            result.control_result_ref.clone(),
            result.load_ref.clone(),
            result.restore_result_ref.clone(),
            result.health_check_ref.clone(),
        ]
        .into_iter()
        .flatten(),
    );
    refs.sort();
    refs.dedup();
    refs
}

fn validation_status_label(validity: &GovernorValidity) -> &'static str {
    match validity {
        GovernorValidity::Measured => "measured",
        GovernorValidity::MeasuredPartial => "measured_partial",
        GovernorValidity::Insufficient => "insufficient",
        GovernorValidity::Refused => "refused",
        GovernorValidity::Contaminated => "contaminated",
        GovernorValidity::NotApplicable => "not_applicable",
        GovernorValidity::Unknown => "unknown",
    }
}

fn validation_coverage_status(
    result: &crate::run_validation::GovernorValidation,
    legacy_identity: bool,
) -> String {
    if legacy_identity {
        return "insufficient".to_string();
    }
    validation_status_label(&result.validity).to_string()
}

fn validation_reason(
    result: &crate::run_validation::GovernorValidation,
    legacy_identity: bool,
) -> String {
    if legacy_identity {
        return crate::run_validation::LEGACY_RUN_VALIDATION_MISSING_RUN_SET_ID.to_string();
    }
    if result.messages.is_empty() {
        return format!(
            "run validation classified governor evidence as {}",
            validation_status_label(&result.validity)
        );
    }
    result.messages.join("; ")
}

fn validation_next_evidence(
    result: &crate::run_validation::GovernorValidation,
    legacy_identity: bool,
) -> Vec<String> {
    if legacy_identity {
        return vec!["rerun report validate-run with v0.2.3+ run-set identity".to_string()];
    }
    if result.next_evidence.is_empty() {
        return vec![
            "rerun governor sweep so plan, approval, apply, load, restore, and health-check evidence are linked"
                .to_string(),
        ];
    }
    result.next_evidence.clone()
}

fn ensure_fixed_frequency_blocked(blocked_points: &mut Vec<RunOperatingPointBlocked>) {
    if blocked_points
        .iter()
        .any(|point| point.factor_id == "fixed_cpu_frequency")
    {
        return;
    }
    blocked_points.push(RunOperatingPointBlocked {
        factor_id: "fixed_cpu_frequency".to_string(),
        requested_level: Some("all_fixed_cpu_frequencies".to_string()),
        coverage_status: "not_controllable".to_string(),
        reason: "observed frequency variation is not a controlled fixed-frequency sweep"
            .to_string(),
        next_evidence: vec![
            "approved privileged frequency control".to_string(),
            "controlled operating point matrix with fixed-frequency levels".to_string(),
            "restore verification for every controlled point".to_string(),
        ],
    });
}

fn blocked_status_for_factor(factor_id: &str) -> &'static str {
    if is_safety_blocked_factor(factor_id) {
        "blocked_unsafe"
    } else {
        "not_controllable"
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
    observed_points: &[RunOperatingPointObserved],
    controlled_points: &[RunOperatingPointControlled],
    blocked_points: &[RunOperatingPointBlocked],
) -> &'static str {
    if blocked_points
        .iter()
        .any(|point| point.coverage_status == "blocked_unsafe")
    {
        "blocked_unsafe"
    } else if !controlled_points.is_empty() {
        "controlled_subset"
    } else if !observed_points.is_empty() {
        "observational_only"
    } else {
        "not_controllable"
    }
}

fn run_report_evaluations(
    run_dir: &Path,
    summary: &RunEvidenceSummary,
    operating_point: &RunOperatingPointSummary,
) -> LabResult<Vec<RuleEvaluation>> {
    let experiment_ref =
        artifact_ref_if_exists(run_dir, &summary.run_id, "experiments/experiment_run.json")?;
    let experiment_completed =
        experiment_has_completed_trial(run_dir.join("experiments/experiment_run.json"))?;
    let bounded_load_completed = !summary.load_result_refs.is_empty();
    let mut evaluations = vec![
        evaluation(
            "report.run.target_inventory_collected",
            claim::RUN_TARGET_INVENTORY_COLLECTED,
            summary.target_inventory_ref.is_some(),
            (Decision::Supported, Decision::Blocked),
            summary.target_inventory_ref.clone().into_iter().collect(),
            &["read-only target inventory"],
            &["collect read-only target inventory"],
        ),
        evaluation(
            "report.run.toolchain_inventory_collected",
            claim::RUN_TOOLCHAIN_INVENTORY_COLLECTED,
            summary.toolchain_inventory_ref.is_some(),
            (Decision::Supported, Decision::Blocked),
            summary
                .toolchain_inventory_ref
                .clone()
                .into_iter()
                .collect(),
            &["read-only toolchain inventory"],
            &["collect read-only toolchain inventory"],
        ),
        evaluation(
            "report.run.tool_qualification_summary_generated",
            claim::RUN_TOOL_QUALIFICATION_SUMMARY_GENERATED,
            summary.tool_qualification_summary_ref.is_some(),
            (Decision::Supported, Decision::Blocked),
            summary
                .tool_qualification_summary_ref
                .clone()
                .into_iter()
                .collect(),
            &["tool qualification summary"],
            &["generate tool qualification summary"],
        ),
        evaluation(
            "report.run.passive_observation_collected",
            claim::RUN_PASSIVE_OBSERVATION_COLLECTED,
            summary.observation_ref.is_some(),
            (Decision::Provisional, Decision::Blocked),
            summary.observation_ref.clone().into_iter().collect(),
            &["passive observation"],
            &[
                "collect passive observation",
                "run controlled operating point matrix before stronger claims",
            ],
        ),
        evaluation(
            "report.run.bounded_load_completed",
            claim::RUN_BOUNDED_LOAD_COMPLETED,
            bounded_load_completed,
            (Decision::Supported, Decision::Blocked),
            summary.load_result_refs.clone(),
            &["bounded CPU load result"],
            &["run bounded CPU load with safety monitor"],
        ),
        evaluation(
            "report.run.operating_point_bounded_workload_measured",
            claim::OPERATING_POINT_BOUNDED_WORKLOAD_MEASURED,
            !operating_point.controlled_points.is_empty(),
            (Decision::Supported, Decision::Blocked),
            operating_point
                .controlled_points
                .iter()
                .flat_map(|point| point.evidence_refs.clone())
                .collect(),
            &["completed bounded workload operating point"],
            &["execute a supported controlled workload matrix"],
        ),
        evaluation(
            "report.run.fixed_cpu_frequency_verified",
            claim::OPERATING_POINT_FIXED_CPU_FREQUENCY_VERIFIED,
            false,
            (Decision::Supported, Decision::Blocked),
            Vec::new(),
            &["fixed CPU frequency control evidence"],
            &[
                "approved privileged control plan",
                "controlled fixed-frequency matrix",
                "restore verification per point",
            ],
        ),
        evaluation(
            "report.run.all_operating_points_measured",
            claim::OPERATING_POINT_ALL_POINTS_MEASURED,
            operating_point.coverage_status == "controlled_full",
            (Decision::Supported, Decision::Blocked),
            Vec::new(),
            &["controlled full operating point coverage"],
            &["controlled operating point matrix across required factors"],
        ),
        evaluation(
            "report.run.sustained_thermal_soak_measured",
            claim::THERMAL_SUSTAINED_SOAK,
            false,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &["sustained thermal soak evidence"],
            &["run approved sustained thermal soak with cooldown observation"],
        ),
        evaluation(
            "report.run.observer_effect_bounded",
            claim::OBSERVER_CADENCE_BOUNDED,
            false,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &["observer effect calibration"],
            &["run observer-off/on workload comparison"],
        ),
        evaluation(
            "report.run.battery_safe_measured",
            claim::BATTERY_SAFE,
            false,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &["target-local power evidence"],
            &["collect target-local power or battery discharge evidence"],
        ),
        evaluation(
            "report.run.real_time_pressure_safe_measured",
            claim::REAL_TIME_PRESSURE_SAFE,
            false,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &["pressure-specific jitter evidence"],
            &["run pressure-specific jitter probes"],
        ),
        evaluation(
            "report.run.production_ready",
            claim::PRODUCTION_READY,
            false,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &["production operating envelope"],
            &[
                "define production operating envelope",
                "run controlled long-duration validation",
                "record recovery and degradation behavior",
            ],
        ),
    ];

    if experiment_ref.is_some() {
        evaluations.push(evaluation(
            "report.run.experiment_bounded_matrix_executed",
            claim::RUN_EXPERIMENT_BOUNDED_MATRIX_EXECUTED,
            experiment_completed,
            (Decision::Supported, Decision::Blocked),
            experiment_ref.into_iter().collect(),
            &["completed experiment trial"],
            &["execute a supported cpu_load_workers matrix"],
        ));
    }
    if summary
        .operations_summary
        .get(OP_CONTROLLED_OPERATING_POINT)
        .map(String::as_str)
        == Some(STATUS_COMPLETED)
        && summary
            .operations_summary
            .get(OP_BOUNDED_LOAD)
            .map(String::as_str)
            != Some(STATUS_COMPLETED)
    {
        evaluations.push(evaluation(
            "report.run.controlled_operating_point_without_root_load",
            claim::RUN_BOUNDED_LOAD_COMPLETED,
            true,
            (Decision::Provisional, Decision::Blocked),
            Vec::new(),
            &[],
            &["repeat controlled trial with bounded load artifact"],
        ));
    }
    Ok(evaluations)
}

fn evaluation(
    rule_id: &str,
    claim_id: &str,
    matched: bool,
    decisions: (Decision, Decision),
    evidence_refs: Vec<String>,
    missing_on_miss: &[&str],
    next_evidence: &[&str],
) -> RuleEvaluation {
    let (on_match, on_miss) = decisions;
    RuleEvaluation {
        rule_id: rule_id.to_string(),
        claim_id: claim_id.to_string(),
        matched,
        decision: if matched { on_match } else { on_miss },
        evidence_refs,
        missing: if matched {
            Vec::new()
        } else {
            missing_on_miss
                .iter()
                .map(|item| (*item).to_string())
                .collect()
        },
        next_evidence: next_evidence
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
    }
}

fn claim_for_run_report_evaluation(evaluation: &RuleEvaluation) -> Claim {
    let mut next_evidence = claim_definition(&evaluation.claim_id)
        .map(|definition| {
            definition
                .default_next_evidence
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    next_evidence.extend(evaluation.next_evidence.clone());
    next_evidence.sort();
    next_evidence.dedup();
    Claim {
        claim_id: evaluation.claim_id.clone(),
        decision: evaluation.decision.clone(),
        evidence_refs: evaluation.evidence_refs.clone(),
        next_evidence,
    }
}

fn blocked_claims(evaluations: &[RuleEvaluation]) -> Vec<String> {
    let mut claims = evaluations
        .iter()
        .filter(|evaluation| evaluation.decision == Decision::Blocked)
        .map(|evaluation| evaluation.claim_id.clone())
        .collect::<Vec<_>>();
    claims.sort();
    claims.dedup();
    claims
}

fn next_evidence(evaluations: &[RuleEvaluation]) -> Vec<String> {
    let mut items = evaluations
        .iter()
        .flat_map(|evaluation| evaluation.next_evidence.clone())
        .collect::<Vec<_>>();
    items.sort();
    items.dedup();
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run_validation::{
        GovernorValidation, GovernorValidity, RunValidationPayload, RunValidationVersionSet,
        VersionSkewPolicyResult, FULLSET_PROFILE,
    };
    use crate::workflow::WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023;
    use std::fs;

    #[test]
    fn run_report_preserves_read_only_blocked_claims() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        let store = EvidenceStore::open(std::slice::from_ref(&run_dir)).unwrap();

        let report = evaluate_run_report_v2(&store, &run_dir, "local-target").unwrap();

        assert_eq!(report.kind, Kind::ReportRun);
        assert_eq!(report.payload.report_status, "observational_read_only");
        assert_eq!(
            report.payload.operating_point.coverage_status,
            "observational_only"
        );
        assert!(report.claims.iter().any(|claim| {
            claim.claim_id == claim::OPERATING_POINT_FIXED_CPU_FREQUENCY_VERIFIED
                && claim.decision == Decision::Blocked
        }));
        assert!(report.claims.iter().any(|claim| {
            claim.claim_id == claim::PRODUCTION_READY && claim.decision == Decision::Blocked
        }));
    }

    #[test]
    fn run_report_records_completed_experiment_operating_point() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
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
        let store = EvidenceStore::open(std::slice::from_ref(&run_dir)).unwrap();

        let report = evaluate_run_report_v2(&store, &run_dir, "local-target").unwrap();

        assert_eq!(
            report.payload.operating_point.coverage_status,
            "controlled_subset"
        );
        assert!(report.claims.iter().any(|claim| {
            claim.claim_id == claim::OPERATING_POINT_BOUNDED_WORKLOAD_MEASURED
                && claim.decision == Decision::Supported
        }));
        assert!(report.claims.iter().any(|claim| {
            claim.claim_id == claim::RUN_EXPERIMENT_BOUNDED_MATRIX_EXECUTED
                && claim.decision == Decision::Supported
        }));
    }

    #[test]
    fn run_report_projects_measured_governor_validation_as_controlled_point() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        let mut store = EvidenceStore::open(std::slice::from_ref(&run_dir)).unwrap();
        write_validation_artifact(&mut store, &run_dir, GovernorValidity::Measured);

        let report = evaluate_run_report_v2(&store, &run_dir, "local-target").unwrap();

        assert!(report
            .payload
            .operating_point
            .controlled_points
            .iter()
            .any(|point| point.factor_id == "governor" && point.level == "performance"));
        assert!(!report
            .payload
            .operating_point
            .blocked_points
            .iter()
            .any(|point| point.factor_id == "governor"));
    }

    #[test]
    fn run_report_projects_contaminated_governor_validation_as_blocked_point() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        let mut store = EvidenceStore::open(std::slice::from_ref(&run_dir)).unwrap();
        write_validation_artifact(&mut store, &run_dir, GovernorValidity::Contaminated);

        let report = evaluate_run_report_v2(&store, &run_dir, "local-target").unwrap();

        assert!(!report
            .payload
            .operating_point
            .controlled_points
            .iter()
            .any(|point| point.factor_id == "governor"));
        assert!(report
            .payload
            .operating_point
            .blocked_points
            .iter()
            .any(|point| {
                point.factor_id == "governor"
                    && point.requested_level.as_deref() == Some("performance")
                    && point.coverage_status == "contaminated"
            }));
    }

    #[test]
    fn run_report_blocks_legacy_validation_missing_run_set_identity() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        write_legacy_v022_validation_artifact(&run_dir);
        let store = EvidenceStore::open(std::slice::from_ref(&run_dir)).unwrap();

        let report = evaluate_run_report_v2(&store, &run_dir, "local-target").unwrap();

        assert!(!report
            .payload
            .operating_point
            .controlled_points
            .iter()
            .any(|point| point.factor_id == "governor"));
        assert!(report
            .payload
            .operating_point
            .blocked_points
            .iter()
            .any(|point| {
                point.factor_id == "governor"
                    && point.requested_level.as_deref() == Some("performance")
                    && point.coverage_status == "insufficient"
                    && point.reason
                        == crate::run_validation::LEGACY_RUN_VALIDATION_MISSING_RUN_SET_ID
            }));
    }

    fn create_completed_core_artifacts(run_dir: &Path) {
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::create_dir_all(run_dir.join("toolchain")).unwrap();
        fs::create_dir_all(run_dir.join("tools")).unwrap();
        fs::create_dir_all(run_dir.join("observations")).unwrap();
        fs::write(run_dir.join("inventory/target_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("toolchain/toolchain_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("tools/tool_qualification_summary.json"), "{}").unwrap();
        fs::write(run_dir.join("observations/observe.json"), "{}").unwrap();
        fs::write(
            run_dir.join("audit.jsonl"),
            format!(
                "{}\n",
                [
                    audit_line("LAB-RUN-001", "inventory"),
                    audit_line("LAB-RUN-001", "toolchain.discover"),
                    audit_line("LAB-RUN-001", "tool.qualify_inventory"),
                    audit_line("LAB-RUN-001", "observe"),
                ]
                .join("\n"),
            ),
        )
        .unwrap();
    }

    fn audit_line(run_id: &str, operation: &str) -> String {
        serde_json::json!({
            "schema_version": "lab.audit_event.v1",
            "event_id": format!("EVT-{operation}"),
            "run_id": run_id,
            "target_id": "local-target",
            "actor": {"kind": "codex", "id": "codex"},
            "operation": operation,
            "operation_id": null,
            "risk_tier": "tier0_read_only_observation",
            "approval_ref": null,
            "restore_lease_ref": null,
            "result": "ok",
            "policy_version": "test",
            "time_unix_ms": 1
        })
        .to_string()
    }

    fn write_legacy_v022_validation_artifact(run_dir: &Path) {
        fs::create_dir_all(run_dir.join("reports")).unwrap();
        fs::write(
            run_dir.join("reports/run_validation.v2.json"),
            serde_json::json!({
                "schema": "lab.artifact.v2",
                "kind": "report.run_validation",
                "id": "RUN-VALIDATION-LEGACY",
                "run_id": "LAB-RUN-001",
                "target_id": "local-target",
                "status": {"state": "measured"},
                "bounds": null,
                "factors": {"controlled": [], "observed": [], "confounders": []},
                "metrics": [],
                "claims": [],
                "evidence_refs": [],
                "data_quality": {"level": "complete", "notes": []},
                "payload": {
                    "profile": FULLSET_PROFILE,
                    "requested_governors": ["performance"],
                    "governor_results": [{
                        "governor": "performance",
                        "validity": "measured",
                        "plan_ref": "artifact://lab/runs/LAB-RUN-001/plans/performance.json",
                        "approval_ref": "artifact://lab/runs/LAB-RUN-001/approvals/performance.json",
                        "control_result_ref": "artifact://lab/runs/LAB-RUN-001/plans/performance.result.json",
                        "load_ref": "artifact://lab/runs/LAB-RUN-001/load/performance.v2.json",
                        "restore_result_ref": "artifact://lab/runs/LAB-RUN-001/restore/performance.result.json",
                        "health_check_ref": "artifact://lab/runs/LAB-RUN-001/health/restore_health_check.json",
                        "messages": ["legacy v0.2.2 validation"],
                        "next_evidence": []
                    }],
                    "overall_validity": "measured",
                    "gaps": [],
                    "audit_refs": ["artifact://lab/runs/LAB-RUN-001/audit.jsonl"]
                },
                "time_unix_ms": 1
            })
            .to_string(),
        )
        .unwrap();
    }

    fn write_validation_artifact(
        store: &mut EvidenceStore,
        run_dir: &Path,
        validity: GovernorValidity,
    ) {
        let status = match validity {
            GovernorValidity::Measured => Status::Measured,
            GovernorValidity::MeasuredPartial => Status::MeasuredPartial,
            GovernorValidity::Refused => Status::Refused {
                code: crate::evidence::EvidenceRefusalCode::PolicyViolation,
                message: "refused control evidence".to_string(),
            },
            GovernorValidity::Contaminated => Status::UnsafeBlocked {
                reason: "contaminated control evidence".to_string(),
            },
            GovernorValidity::NotApplicable => Status::NotApplicable {
                reason: "not applicable".to_string(),
            },
            GovernorValidity::Insufficient | GovernorValidity::Unknown => Status::Insufficient,
        };
        let artifact = Artifact::new(
            Kind::ReportRunValidation,
            "RUN-VALIDATION-001",
            "LAB-RUN-001",
            "local-target",
            status,
            RunValidationPayload {
                profile: FULLSET_PROFILE.to_string(),
                requested_governors: vec!["performance".to_string()],
                workflow_recommendation_ref: None,
                collect_plan_ref: None,
                collect_plan_digest: None,
                subject_run_set_id: "RUN-SET-test".to_string(),
                included_run_refs: vec!["artifact://lab/runs/LAB-RUN-001/".to_string()],
                validation_profile: FULLSET_PROFILE.to_string(),
                workflow_id: WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023.to_string(),
                expected_governors: vec!["performance".to_string()],
                target_id: "local-target".to_string(),
                target_class: "raspberry_pi_4".to_string(),
                version_set: RunValidationVersionSet {
                    records: Vec::new(),
                    skew_detected: false,
                    skew_reasons: Vec::new(),
                },
                version_skew_policy: VersionSkewPolicyResult::NoSkewDetected,
                version_skew_override: false,
                governor_results: vec![GovernorValidation {
                    governor: "performance".to_string(),
                    validity: validity.clone(),
                    plan_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/plans/performance.json".to_string(),
                    ),
                    approval_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/approvals/performance.json".to_string(),
                    ),
                    control_result_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/plans/performance.result.json".to_string(),
                    ),
                    load_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/load/performance.v2.json".to_string(),
                    ),
                    restore_result_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/restore/performance.result.json"
                            .to_string(),
                    ),
                    health_check_ref: Some(
                        "artifact://lab/runs/LAB-RUN-001/health/restore_health_check.json"
                            .to_string(),
                    ),
                    messages: vec!["test validation".to_string()],
                    next_evidence: vec!["rerun governor sweep".to_string()],
                }],
                overall_validity: validity,
                gaps: Vec::new(),
                audit_refs: vec!["artifact://lab/runs/LAB-RUN-001/audit.jsonl".to_string()],
            },
            1,
        );
        store
            .write(
                run_dir,
                Path::new("reports/run_validation.v2.json"),
                &artifact,
            )
            .unwrap();
    }
}
