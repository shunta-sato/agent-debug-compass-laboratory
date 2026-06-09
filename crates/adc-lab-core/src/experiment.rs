use crate::contracts::{
    ClaimDecision, ClaimEvidenceTrace, ClaimTraceEntry, ExperimentMatrix, ExperimentRun,
    ExperimentTrial,
};
use crate::ids::{new_id, now_unix_ms};
use std::collections::BTreeMap;

pub fn run_experiment_matrix(
    matrix: &ExperimentMatrix,
    run_id: String,
    target_id: String,
    dry_run: bool,
) -> (ExperimentRun, ClaimEvidenceTrace) {
    let combinations = expand_factors(matrix);
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
                evidence_refs: vec!["artifact://experiment_run".to_string()],
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
    (run, trace)
}

fn expand_factors(matrix: &ExperimentMatrix) -> Vec<BTreeMap<String, String>> {
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
    if combinations.is_empty() {
        vec![BTreeMap::new()]
    } else {
        combinations
    }
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
            run_experiment_matrix(&matrix, "LAB-RUN-001".to_string(), "pi4".to_string(), true);
        assert_eq!(run.trials.len(), 4);
        assert_eq!(trace.claims.len(), 2);
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
            run_experiment_matrix(&matrix, "LAB-RUN-001".to_string(), "pi4".to_string(), false);
        assert_eq!(run.trials[0].status, "not_implemented");
        assert!(trace
            .claims
            .iter()
            .all(|claim| claim.decision != ClaimDecision::Supported));
    }
}
