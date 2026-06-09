use crate::contracts::{
    ClaimDecision, ClaimEvidenceTrace, ClaimTraceEntry, ExperimentMatrix, ExperimentRun,
    ExperimentTrial,
};
use crate::ids::{new_id, now_unix_ms};
use crate::{LabError, LabResult};
use std::collections::BTreeMap;

pub const MAX_EXPERIMENT_WARMUP_SECONDS: u64 = 60;
pub const MAX_EXPERIMENT_COOLDOWN_SECONDS: u64 = 60;
pub const MAX_EXPERIMENT_REPETITIONS: u64 = 10;
pub const MAX_EXPERIMENT_TRIALS: usize = 64;

pub fn run_experiment_matrix(
    matrix: &ExperimentMatrix,
    run_id: String,
    target_id: String,
    dry_run: bool,
) -> LabResult<(ExperimentRun, ClaimEvidenceTrace)> {
    validate_experiment_matrix_bounds(matrix)?;
    let combinations = expand_factors(matrix)?;
    let experiment_ref = format!("artifact://lab/runs/{run_id}/experiments/experiment_run.json");
    let mut trials = Vec::new();
    for repetition in 0..matrix.repetitions.max(1) {
        for factors in &combinations {
            trials.push(ExperimentTrial {
                trial_id: format!("TRIAL-{}-{repetition}", new_id("MATRIX")),
                factors: factors.clone(),
                status: if dry_run {
                    "planned".to_string()
                } else {
                    "not_implemented".to_string()
                },
                artifact_refs: Vec::new(),
                failure: (!dry_run).then_some(
                    "experiment execution is not implemented by this planner".to_string(),
                ),
                started_at_unix_ms: None,
                ended_at_unix_ms: None,
            });
        }
    }

    let run = ExperimentRun {
        schema_version: "lab.experiment_run.v1".to_string(),
        run_id: run_id.clone(),
        matrix_id: matrix.matrix_id.clone(),
        target_id: target_id.clone(),
        dry_run,
        trials,
        time_unix_ms: now_unix_ms(),
    };
    let trace = ClaimEvidenceTrace {
        schema_version: "lab.claim_evidence_trace.v1".to_string(),
        run_id,
        target_id,
        claims: vec![
            ClaimTraceEntry {
                claim: if dry_run {
                    "Frequency variation is planned under bounded matrix conditions.".to_string()
                } else {
                    "Experiment execution is not implemented in this MVP.".to_string()
                },
                decision: if dry_run {
                    ClaimDecision::Provisional
                } else {
                    ClaimDecision::Blocked
                },
                evidence_refs: vec![experiment_ref],
                next_evidence_needed: if dry_run {
                    vec!["Run matrix on a characterized target with approved controls.".to_string()]
                } else {
                    vec![
                        "Wire matrix execution to audited control/load/observe steps before making supported claims."
                            .to_string(),
                    ]
                },
            },
            ClaimTraceEntry {
                claim: "adc-lab verified behavior across all fixed CPU frequencies.".to_string(),
                decision: ClaimDecision::Blocked,
                evidence_refs: Vec::new(),
                next_evidence_needed: vec![
                    "Controlled operating point matrix with fixed frequency support.".to_string(),
                ],
            },
        ],
        time_unix_ms: now_unix_ms(),
    };
    Ok((run, trace))
}

pub fn expand_factors(matrix: &ExperimentMatrix) -> LabResult<Vec<BTreeMap<String, String>>> {
    let mut combinations = vec![BTreeMap::new()];
    for factor in &matrix.factors {
        let mut next = Vec::new();
        for existing in &combinations {
            for level in &factor.levels {
                let mut entry = existing.clone();
                entry.insert(factor.name.clone(), level.clone());
                next.push(entry);
            }
        }
        combinations = next;
    }
    let combinations = if combinations.is_empty() {
        vec![BTreeMap::new()]
    } else {
        combinations
    };
    let total = combinations.len() * matrix.repetitions.max(1) as usize;
    if total > MAX_EXPERIMENT_TRIALS {
        return Err(LabError::Policy(format!(
            "experiment expands to {total} trials, max is {MAX_EXPERIMENT_TRIALS}"
        )));
    }
    Ok(combinations)
}

pub fn validate_experiment_matrix_bounds(matrix: &ExperimentMatrix) -> LabResult<()> {
    if matrix.warmup_seconds > MAX_EXPERIMENT_WARMUP_SECONDS {
        return Err(LabError::Policy(format!(
            "experiment warmup must be <= {MAX_EXPERIMENT_WARMUP_SECONDS}s"
        )));
    }
    if matrix.cooldown_seconds > MAX_EXPERIMENT_COOLDOWN_SECONDS {
        return Err(LabError::Policy(format!(
            "experiment cooldown must be <= {MAX_EXPERIMENT_COOLDOWN_SECONDS}s"
        )));
    }
    if matrix.repetitions == 0 || matrix.repetitions > MAX_EXPERIMENT_REPETITIONS {
        return Err(LabError::Policy(format!(
            "experiment repetitions must be between 1 and {MAX_EXPERIMENT_REPETITIONS}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ExperimentFactor, FactorKind};

    #[test]
    fn contract_validation_matrix_expands_trials() {
        let matrix = ExperimentMatrix {
            schema_version: "lab.experiment_matrix.v1".to_string(),
            matrix_id: "MATRIX-001".to_string(),
            description: "smoke".to_string(),
            factors: vec![ExperimentFactor {
                name: "governor".to_string(),
                kind: FactorKind::ControlledFactor,
                levels: vec!["ondemand".to_string(), "performance".to_string()],
            }],
            warmup_seconds: 0,
            cooldown_seconds: 0,
            repetitions: 2,
            order: "listed".to_string(),
        };
        let (run, trace) =
            run_experiment_matrix(&matrix, "LAB-RUN-001".to_string(), "pi4".to_string(), true)
                .unwrap();
        assert_eq!(run.trials.len(), 4);
        assert_eq!(trace.claims.len(), 2);
        assert!(run
            .trials
            .iter()
            .all(|trial| trial.artifact_refs.is_empty()));
    }

    #[test]
    fn contract_validation_non_dry_matrix_does_not_claim_execution() {
        let matrix = ExperimentMatrix {
            schema_version: "lab.experiment_matrix.v1".to_string(),
            matrix_id: "MATRIX-001".to_string(),
            description: "smoke".to_string(),
            factors: vec![ExperimentFactor {
                name: "governor".to_string(),
                kind: FactorKind::ControlledFactor,
                levels: vec!["ondemand".to_string()],
            }],
            warmup_seconds: 0,
            cooldown_seconds: 0,
            repetitions: 1,
            order: "listed".to_string(),
        };
        let (run, trace) =
            run_experiment_matrix(&matrix, "LAB-RUN-001".to_string(), "pi4".to_string(), false)
                .unwrap();
        assert_eq!(run.trials[0].status, "not_implemented");
        assert!(run.trials[0].failure.is_some());
        assert!(trace
            .claims
            .iter()
            .all(|claim| claim.decision != ClaimDecision::Supported));
    }

    #[test]
    fn contract_validation_matrix_bounds_reject_trial_explosion() {
        let matrix = ExperimentMatrix {
            schema_version: "lab.experiment_matrix.v1".to_string(),
            matrix_id: "MATRIX-001".to_string(),
            description: "too many trials".to_string(),
            factors: vec![
                ExperimentFactor {
                    name: "a".to_string(),
                    kind: FactorKind::ObservedCovariate,
                    levels: (0..9).map(|index| index.to_string()).collect(),
                },
                ExperimentFactor {
                    name: "b".to_string(),
                    kind: FactorKind::ObservedCovariate,
                    levels: (0..9).map(|index| index.to_string()).collect(),
                },
            ],
            warmup_seconds: 0,
            cooldown_seconds: 0,
            repetitions: 1,
            order: "listed".to_string(),
        };
        let error = expand_factors(&matrix).unwrap_err();
        assert!(error.to_string().contains("trials"));
    }
}
