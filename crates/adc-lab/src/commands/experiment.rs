use super::super::*;
use super::common::*;
use adc_lab_core::ids::{new_id, now_unix_ms};
use anyhow::Context;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub(crate) fn command_experiment_run(args: ExperimentRunCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    persist_target_runner_version_if_absent(&run, &target)?;
    let matrix: ExperimentMatrix = read_yaml(&args.matrix)?;
    let experiment_run = if args.dry_run {
        run_experiment_matrix(&matrix, run.run_id.clone(), target.target_id.clone(), true)?
    } else {
        let config = ExperimentRuntimeConfig {
            load_duration: parse_duration(&args.trial_load_duration)?,
            observe_duration: parse_duration(&args.trial_observe_duration)?,
            sample_interval: parse_duration(&args.trial_sample_interval)?,
            load_abort_temp_c: args.load_abort_temp_c,
            operator_abort_file: args.operator_abort_file.clone(),
        };
        execute_experiment_matrix(&run, &target, &matrix, &config)?
    };
    let audit_result = experiment_run_result(&experiment_run);
    let run_path = run.run_dir.join("experiments/experiment_run.json");
    write_json_artifact(&run, &run_path, &experiment_run)?;
    persist_run_report(&run, target.target_id.clone())?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: target.target_id,
            actor: Actor::codex(),
            operation: "experiment.run".to_string(),
            operation_id: Some(matrix.matrix_id),
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: audit_result,
        },
    )?;
    print_artifact(&run, &run_path, experiment_run)
}

#[derive(Debug, Clone)]
struct ExperimentRuntimeConfig {
    load_duration: Duration,
    observe_duration: Duration,
    sample_interval: Duration,
    load_abort_temp_c: Option<f64>,
    operator_abort_file: Option<PathBuf>,
}

fn execute_experiment_matrix(
    run: &RunContext,
    target: &TargetSpec,
    matrix: &ExperimentMatrix,
    config: &ExperimentRuntimeConfig,
) -> Result<ExperimentRun> {
    validate_experiment_matrix_bounds(matrix)?;
    let combinations = expand_factors(matrix)?;
    let mut trials = Vec::new();
    for repetition in 0..matrix.repetitions.max(1) {
        for factors in &combinations {
            let trial_id = format!("TRIAL-{}-{repetition}", new_id("MATRIX"));
            let mut trial = ExperimentTrial {
                trial_id: trial_id.clone(),
                factors: factors.clone(),
                status: "planned".to_string(),
                artifact_refs: Vec::new(),
                failure: None,
                started_at_unix_ms: Some(now_unix_ms()),
                ended_at_unix_ms: None,
            };

            if let Some(reason) = blocked_trial_reason(matrix, factors) {
                trial.status = "blocked".to_string();
                trial.failure = Some(reason);
            } else {
                sleep_experiment_phase(matrix.warmup_seconds);
                let result = execute_supported_experiment_trial(run, target, &mut trial, config);
                sleep_experiment_phase(matrix.cooldown_seconds);
                if let Err(error) = result {
                    trial.status = "failed".to_string();
                    trial.failure = Some(error.to_string());
                }
            }
            trial.ended_at_unix_ms = Some(now_unix_ms());
            append_audit_event(
                run,
                AuditInput {
                    target_id: target.target_id.clone(),
                    actor: Actor::codex(),
                    operation: "experiment.trial".to_string(),
                    operation_id: Some(trial.trial_id.clone()),
                    risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
                    approval_ref: None,
                    restore_lease_ref: None,
                    result: trial.status.clone(),
                },
            )?;
            trials.push(trial);
        }
    }

    Ok(ExperimentRun {
        schema_version: "lab.experiment_run.v1".to_string(),
        run_id: run.run_id.clone(),
        matrix_id: matrix.matrix_id.clone(),
        target_id: target.target_id.clone(),
        dry_run: false,
        trials,
        time_unix_ms: now_unix_ms(),
    })
}

fn execute_supported_experiment_trial(
    run: &RunContext,
    target: &TargetSpec,
    trial: &mut ExperimentTrial,
    config: &ExperimentRuntimeConfig,
) -> Result<()> {
    if let Some(workers) = cpu_load_workers(&trial.factors)? {
        if workers > 0 {
            let trial_dir = trial_artifact_dir(run, &trial.trial_id);
            let load_result = match target.transport {
                TargetTransport::Local => {
                    let plan = new_cpu_load_plan_with_operator_abort(
                        target.target_id.clone(),
                        workers,
                        config.load_duration,
                        config.load_abort_temp_c,
                        config.operator_abort_file.is_some(),
                    )?;
                    let plan_path = trial_dir.join("load_plan.json");
                    let plan_ref = write_json_artifact(run, &plan_path, &plan)?;
                    trial.artifact_refs.push(plan_ref);
                    run_cpu_load_with_options(
                        &plan,
                        &CpuLoadRuntimeOptions {
                            operator_abort_file: config.operator_abort_file.clone(),
                        },
                    )?
                }
                TargetTransport::Ssh => load_cpu_ssh(
                    target,
                    workers,
                    config.load_duration,
                    config.load_abort_temp_c,
                    config.operator_abort_file.as_deref(),
                )?,
            };
            let result_status = load_result.status.clone();
            let abort_reason = load_result.abort_reason.clone();
            let relative = PathBuf::from("load").join("experiments").join(format!(
                "{}.{}.v2.json",
                safe_artifact_id(&trial.trial_id, "TRIAL"),
                safe_artifact_id(&load_result.result_id, "LOAD-RESULT")
            ));
            let artifact = load_artifact_v2(run.run_id.clone(), load_result);
            let mut store = evidence_store_for_run(run)?;
            let result_ref = store.write(&run.run_dir, &relative, &artifact)?;
            trial.artifact_refs.push(result_ref);
            if result_status != "completed" {
                anyhow::bail!(
                    "load step ended with status {result_status}: {}",
                    abort_reason.unwrap_or_else(|| "no abort reason".to_string())
                );
            }
        }
    }

    let observation = match target.transport {
        TargetTransport::Local => observe_local(
            target.target_id.clone(),
            config.observe_duration,
            config.sample_interval,
            vec![Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory],
        )?,
        TargetTransport::Ssh => observe_ssh(
            target,
            config.observe_duration,
            config.sample_interval,
            &[Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory],
        )?,
    };
    let observation_path = trial_artifact_dir(run, &trial.trial_id).join("observation.json");
    let observation_ref = write_json_artifact(run, &observation_path, &observation)?;
    trial.artifact_refs.push(observation_ref);
    trial.status = "completed".to_string();
    Ok(())
}

fn blocked_trial_reason(
    matrix: &ExperimentMatrix,
    factors: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    if matrix.order != "listed" {
        return Some("randomized order requires seeded reproducible execution".to_string());
    }
    for factor in &matrix.factors {
        if factor.kind == FactorKind::ControlledFactor && factor.name != "cpu_load_workers" {
            return Some(format!(
                "controlled factor '{}' is not supported by PR6 real runner",
                factor.name
            ));
        }
    }
    if let Err(error) = cpu_load_workers(factors) {
        return Some(error.to_string());
    }
    None
}

fn cpu_load_workers(factors: &std::collections::BTreeMap<String, String>) -> Result<Option<usize>> {
    let Some(raw) = factors.get("cpu_load_workers") else {
        return Ok(None);
    };
    if raw == "none" || raw == "0" {
        return Ok(Some(0));
    }
    let workers = raw
        .parse::<usize>()
        .with_context(|| format!("invalid cpu_load_workers level '{raw}'"))?;
    if workers == 0 {
        Ok(Some(0))
    } else {
        Ok(Some(workers))
    }
}

fn trial_artifact_dir(run: &RunContext, trial_id: &str) -> PathBuf {
    run.run_dir
        .join("experiments")
        .join("trials")
        .join(safe_artifact_id(trial_id, "TRIAL"))
}

fn experiment_run_result(run: &ExperimentRun) -> String {
    if run.dry_run {
        return "planned".to_string();
    }
    if run.trials.iter().any(|trial| trial.status == "failed") {
        "failed".to_string()
    } else if run.trials.iter().any(|trial| trial.status == "blocked") {
        "blocked".to_string()
    } else {
        "completed".to_string()
    }
}

fn sleep_experiment_phase(seconds: u64) {
    if seconds > 0 {
        thread::sleep(Duration::from_secs(seconds));
    }
}
