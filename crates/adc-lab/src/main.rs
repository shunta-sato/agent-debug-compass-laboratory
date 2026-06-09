use adc_lab_core::ids::{new_id, now_unix_ms};
use adc_lab_core::*;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "adc-lab")]
#[command(about = "Safety-gated embedded target familiarization laboratory")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Inventory(TargetCommand),
    Toolchain {
        #[command(subcommand)]
        command: ToolchainCommand,
    },
    Observe(ObserveCommand),
    Control {
        #[command(subcommand)]
        command: ControlCommand,
    },
    Restore(RestoreCommand),
    Load {
        #[command(subcommand)]
        command: LoadCommand,
    },
    Experiment {
        #[command(subcommand)]
        command: ExperimentCommand,
    },
    Familiarize {
        #[command(subcommand)]
        command: FamiliarizeCommand,
    },
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
    HealthCheck(TargetCommand),
    Privilege {
        #[command(subcommand)]
        command: PrivilegeCommand,
    },
    Tool {
        #[command(subcommand)]
        command: ToolCommand,
    },
}

#[derive(Debug, Args)]
struct TargetCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ToolchainCommand {
    Discover(TargetCommand),
}

#[derive(Debug, Subcommand)]
enum PrivilegeCommand {
    #[command(name = "provider-status")]
    ProviderStatus(TargetCommand),
}

#[derive(Debug, Args)]
struct ObserveCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    duration: String,
    #[arg(long, default_value = "1s")]
    sample_interval: String,
    #[arg(long, value_delimiter = ',')]
    signals: Vec<Signal>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ControlCommand {
    Plan(ControlPlanCommand),
    Approve(ControlApproveCommand),
    Apply(ControlApplyCommand),
}

#[derive(Debug, Args)]
struct ControlPlanCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[command(subcommand)]
    operation: ControlPlanOperation,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long, default_value_t = 60)]
    duration_seconds_max: u64,
    #[arg(long)]
    thermal_celsius_abort: Option<f64>,
}

#[derive(Debug, Args)]
struct ControlApproveCommand {
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    approved_by: String,
    #[arg(long)]
    operation_summary: Option<String>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ControlPlanOperation {
    #[command(name = "cpu.governor")]
    CpuGovernor {
        #[arg(long = "set")]
        governor: String,
    },
}

#[derive(Debug, Args)]
struct ControlApplyCommand {
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    approval: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct RestoreCommand {
    #[arg(long)]
    lease: PathBuf,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum LoadCommand {
    Cpu(LoadCpuCommand),
}

#[derive(Debug, Args)]
struct LoadCpuCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long, default_value_t = 1)]
    workers: usize,
    #[arg(long)]
    duration: String,
    #[arg(long)]
    abort_temp_c: Option<f64>,
    #[arg(long)]
    operator_abort_file: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ExperimentCommand {
    Run(ExperimentRunCommand),
}

#[derive(Debug, Subcommand)]
enum FamiliarizeCommand {
    #[command(name = "read-only")]
    ReadOnly(FamiliarizeReadOnlyCommand),
}

#[derive(Debug, Args)]
struct FamiliarizeReadOnlyCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    duration: String,
    #[arg(long, default_value = "1s")]
    sample_interval: String,
    #[arg(long, value_delimiter = ',')]
    signals: Vec<Signal>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ExperimentRunCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    matrix: PathBuf,
    #[arg(long)]
    dry_run: bool,
    #[arg(long, default_value = "1s")]
    trial_load_duration: String,
    #[arg(long, default_value = "0s")]
    trial_observe_duration: String,
    #[arg(long, default_value = "1s")]
    trial_sample_interval: String,
    #[arg(long = "load-abort-temp-c")]
    load_abort_temp_c: Option<f64>,
    #[arg(long)]
    operator_abort_file: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ReportCommand {
    Pack(ReportPackCommand),
    OperatingPoint(ReportPackCommand),
    CapabilityProfile(TargetCapabilityProfileCommand),
}

#[derive(Debug, Args)]
struct ReportPackCommand {
    #[arg(long)]
    run: PathBuf,
    #[arg(long, default_value = "unknown-target")]
    target_id: String,
    #[arg(long, default_value = "unknown-target")]
    target: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct TargetCapabilityProfileCommand {
    #[arg(long)]
    run: PathBuf,
    #[arg(long, default_value = "unknown-target")]
    target_id: String,
    #[arg(long)]
    workload: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ToolCommand {
    Qualify(ToolQualifyCommand),
    QualifyInventory(ToolQualifyInventoryCommand),
}

#[derive(Debug, Args)]
struct ToolQualifyCommand {
    #[arg(long)]
    manifest: PathBuf,
    #[arg(long)]
    tool_version: Option<String>,
    #[arg(long)]
    tool_sha256: Option<String>,
    #[arg(long)]
    output_schema: Option<PathBuf>,
    #[arg(long)]
    dry_run_output: Option<PathBuf>,
    #[arg(long)]
    manual_comparison: Option<PathBuf>,
    #[arg(long)]
    static_safety_review: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolQualifyInventoryCommand {
    #[arg(long)]
    inventory: PathBuf,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactOutput<T: Serialize> {
    artifact_ref: String,
    value: T,
}

#[derive(Debug)]
struct PendingToolQualificationEvidence {
    evidence: ToolQualificationEvidence,
    output_schema_path: PathBuf,
    output_schema: serde_json::Value,
    dry_run_path: PathBuf,
    dry_run: serde_json::Value,
    manual_comparison_path: PathBuf,
    manual_comparison: serde_json::Value,
    static_safety_review_path: PathBuf,
    static_safety_review: String,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadOnlyFamiliarizeOutput {
    run_id: String,
    target_id: String,
    run_manifest_ref: String,
    familiarization_pack_ref: String,
    claim_trace_ref: String,
    value: FamiliarizationPack,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct HealthOutput {
    schema_version: String,
    target_id: String,
    status: String,
    inventory_available: bool,
    toolchain_available: bool,
}

fn main() -> Result<()> {
    if print_version_if_requested("adc-lab")? {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Inventory(args) => command_inventory(args),
        Commands::Toolchain { command } => match command {
            ToolchainCommand::Discover(args) => command_toolchain_discover(args),
        },
        Commands::Observe(args) => command_observe(args),
        Commands::Control { command } => match command {
            ControlCommand::Plan(args) => command_control_plan(args),
            ControlCommand::Approve(args) => command_control_approve(args),
            ControlCommand::Apply(args) => command_control_apply(args),
        },
        Commands::Restore(args) => command_restore(args),
        Commands::Load { command } => match command {
            LoadCommand::Cpu(args) => command_load_cpu(args),
        },
        Commands::Experiment { command } => match command {
            ExperimentCommand::Run(args) => command_experiment_run(args),
        },
        Commands::Familiarize { command } => match command {
            FamiliarizeCommand::ReadOnly(args) => command_familiarize_read_only(args),
        },
        Commands::Report { command } => match command {
            ReportCommand::Pack(args) => command_report_pack(args),
            ReportCommand::OperatingPoint(args) => command_report_operating_point(args),
            ReportCommand::CapabilityProfile(args) => command_report_capability_profile(args),
        },
        Commands::HealthCheck(args) => command_health_check(args),
        Commands::Privilege { command } => match command {
            PrivilegeCommand::ProviderStatus(args) => command_privilege_provider_status(args),
        },
        Commands::Tool { command } => match command {
            ToolCommand::Qualify(args) => command_tool_qualify(args),
            ToolCommand::QualifyInventory(args) => command_tool_qualify_inventory(args),
        },
    }
}

fn print_version_if_requested(name: &str) -> Result<bool> {
    let Some(arg) = std::env::args_os().nth(1) else {
        return Ok(false);
    };
    if arg == "--version" || arg == "-V" {
        print_json(&build_info(name))?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn command_inventory(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let inventory = collect_inventory(&target)?;
    let path = run.run_dir.join("inventory/target_inventory.json");
    write_json_artifact(&run, &path, &inventory)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: inventory.target_id.clone(),
            actor: Actor::codex(),
            operation: "inventory".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    print_artifact(&run, &path, inventory)
}

fn command_toolchain_discover(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let inventory = discover_toolchain(&target)?;
    let path = run.run_dir.join("toolchain/toolchain_inventory.json");
    write_json_artifact(&run, &path, &inventory)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: inventory.target_id.clone(),
            actor: Actor::codex(),
            operation: "toolchain.discover".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    print_artifact(&run, &path, inventory)
}

fn command_observe(args: ObserveCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let duration = parse_duration(&args.duration)?;
    let interval = parse_duration(&args.sample_interval)?;
    let signals = if args.signals.is_empty() {
        vec![Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory]
    } else {
        args.signals
    };
    let observation = match target.transport {
        TargetTransport::Local => {
            observe_local(target.target_id.clone(), duration, interval, signals)?
        }
        TargetTransport::Ssh => observe_ssh(&target, duration, interval, &signals)?,
    };
    let path = run.run_dir.join("observations/observe.json");
    write_json_artifact(&run, &path, &observation)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: observation.target_id.clone(),
            actor: Actor::codex(),
            operation: "observe".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    print_artifact(&run, &path, observation)
}

fn command_control_plan(args: ControlPlanCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let governor = match args.operation {
        ControlPlanOperation::CpuGovernor { governor } => governor,
    };
    let plan = new_cpufreq_plan(
        &run,
        &target,
        governor,
        args.duration_seconds_max,
        args.thermal_celsius_abort,
    );
    if let Err(refusal) = validate_control_plan(&plan) {
        anyhow::bail!("invalid generated plan: {}", refusal.message);
    }
    let path = run
        .run_dir
        .join("plans")
        .join(format!("{}.json", plan.plan_id));
    write_json_artifact(&run, &path, &plan)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: plan.target_id.clone(),
            actor: Actor::codex(),
            operation: "control.plan".to_string(),
            operation_id: Some(plan.operation.operation_id.clone()),
            risk_tier: plan.risk_tier.clone(),
            approval_ref: None,
            restore_lease_ref: None,
            result: "planned".to_string(),
        },
    )?;
    print_artifact(&run, &path, plan)
}

fn command_control_approve(args: ControlApproveCommand) -> Result<()> {
    let plan: ControlPlan = read_json(&args.plan)?;
    if plan.target_id != LOCAL_TARGET_ID {
        anyhow::bail!(
            "control approval is local-target only in this MVP; refused target_id={}",
            plan.target_id
        );
    }
    let run = create_or_open_run(args.run_dir.or_else(|| infer_run_dir(&args.plan)))?;
    let summary = args.operation_summary.unwrap_or_else(|| {
        format!(
            "Approve {} for target {}",
            plan.operation.operation_id, plan.target_id
        )
    });
    let approval = new_approval_record(&plan, args.approved_by, summary)?;
    let (path, artifact_ref) = write_approval_record(&run, &approval)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: approval.target_id.clone(),
            actor: Actor::codex(),
            operation: "control.approve".to_string(),
            operation_id: Some(approval.approved_operation.operation_id.clone()),
            risk_tier: approval.risk_tier.clone(),
            approval_ref: Some(artifact_ref),
            restore_lease_ref: None,
            result: "approved".to_string(),
        },
    )?;
    print_artifact(&run, &path, approval)
}

fn command_control_apply(args: ControlApplyCommand) -> Result<()> {
    let plan: ControlPlan = read_json(&args.plan)?;
    let run = create_or_open_run(args.run_dir.or_else(|| infer_run_dir(&args.plan)))?;
    let mut approval_ref = None;
    let result = if plan.target_id != LOCAL_TARGET_ID {
        refused_result(&plan, target_local_helper_refusal(&plan.target_id))
    } else {
        let approval = args.approval.as_ref().map(read_json).transpose()?;
        approval_ref = approval
            .as_ref()
            .map(|approval| persist_approval_record(&run, approval))
            .transpose()?;
        let dry_run_result = apply_control_plan(
            &plan,
            approval.as_ref(),
            &LinuxCpufreqBackend::default(),
            true,
        );
        if args.dry_run || dry_run_result.status == ControlResultStatus::Refused {
            dry_run_result
        } else {
            invoke_helper_apply(&args.plan, args.approval.as_deref())?
        }
    };
    persist_control_result(&run, &result, approval_ref)?;
    print_json(&result)
}

fn command_restore(args: RestoreCommand) -> Result<()> {
    let lease: RestoreLease = read_json(&args.lease)?;
    let run = create_or_open_run(args.run_dir.or_else(|| infer_run_dir(&args.lease)))?;
    let result = if lease.target_id != LOCAL_TARGET_ID {
        restore_refused_result(&lease, target_local_helper_refusal(&lease.target_id))
    } else if args.dry_run {
        restore_lease(&lease, &LinuxCpufreqBackend::default(), true)
    } else {
        invoke_helper_restore(&args.lease)?
    };
    persist_control_result(&run, &result, None)?;
    if result.status == ControlResultStatus::Restored && result.target_id == LOCAL_TARGET_ID {
        persist_restore_health_check(&run)?;
    }
    print_json(&result)
}

fn command_load_cpu(args: LoadCpuCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let duration = parse_duration(&args.duration)?;
    let result = match target.transport {
        TargetTransport::Local => {
            let plan = new_cpu_load_plan_with_operator_abort(
                target.target_id.clone(),
                args.workers,
                duration,
                args.abort_temp_c,
                args.operator_abort_file.is_some(),
            )?;
            let plan_path = run
                .run_dir
                .join("loads")
                .join(format!("{}.plan.json", plan.load_id));
            write_json_artifact(&run, &plan_path, &plan)?;
            run_cpu_load_with_options(
                &plan,
                &CpuLoadRuntimeOptions {
                    operator_abort_file: args.operator_abort_file.clone(),
                },
            )?
        }
        TargetTransport::Ssh => load_cpu_ssh(
            &target,
            args.workers,
            duration,
            args.abort_temp_c,
            args.operator_abort_file.as_deref(),
        )?,
    };
    let path = run
        .run_dir
        .join("loads")
        .join(format!("{}.result.json", result.load_id));
    write_json_artifact(&run, &path, &result)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: result.target_id.clone(),
            actor: Actor::codex(),
            operation: "load.cpu".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: result.status.clone(),
        },
    )?;
    print_artifact(&run, &path, result)
}

fn command_experiment_run(args: ExperimentRunCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let matrix: ExperimentMatrix = read_yaml(&args.matrix)?;
    let (experiment_run, claim_trace) = if args.dry_run {
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
    let trace_path = run.run_dir.join("reports/claim_evidence_trace.json");
    write_json_artifact(&run, &run_path, &experiment_run)?;
    write_json_artifact(&run, &trace_path, &claim_trace)?;
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
) -> Result<(ExperimentRun, ClaimEvidenceTrace)> {
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

    let experiment_run = ExperimentRun {
        schema_version: "lab.experiment_run.v1".to_string(),
        run_id: run.run_id.clone(),
        matrix_id: matrix.matrix_id.clone(),
        target_id: target.target_id.clone(),
        dry_run: false,
        trials,
        time_unix_ms: now_unix_ms(),
    };
    let claim_trace = real_experiment_claim_trace(&experiment_run);
    Ok((experiment_run, claim_trace))
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
            let result_path = trial_dir.join("load_result.json");
            let result_status = load_result.status.clone();
            let abort_reason = load_result.abort_reason.clone();
            let result_ref = write_json_artifact(run, &result_path, &load_result)?;
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

fn real_experiment_claim_trace(run: &ExperimentRun) -> ClaimEvidenceTrace {
    let experiment_ref = format!(
        "artifact://lab/runs/{}/experiments/experiment_run.json",
        run.run_id
    );
    let completed = run
        .trials
        .iter()
        .filter(|trial| trial.status == "completed")
        .count();
    let blocked_or_failed = run
        .trials
        .iter()
        .filter(|trial| trial.status == "blocked" || trial.status == "failed")
        .count();
    ClaimEvidenceTrace {
        schema_version: "lab.claim_evidence_trace.v1".to_string(),
        run_id: run.run_id.clone(),
        target_id: run.target_id.clone(),
        claims: vec![
            ClaimTraceEntry {
                claim: format!(
                    "Bounded non-privileged experiment matrix executed {completed} completed trial(s)."
                ),
                decision: if completed > 0 {
                    ClaimDecision::Supported
                } else {
                    ClaimDecision::Blocked
                },
                evidence_refs: if completed > 0 {
                    vec![experiment_ref.clone()]
                } else {
                    Vec::new()
                },
                next_evidence_needed: if completed > 0 {
                    vec![
                        "controlled privileged factors require approval/apply/restore wiring"
                            .to_string(),
                    ]
                } else {
                    vec!["execute a supported cpu_load_workers matrix".to_string()]
                },
            },
            ClaimTraceEntry {
                claim: format!(
                    "{blocked_or_failed} trial(s) were blocked or failed and cannot support claims."
                ),
                decision: if blocked_or_failed > 0 {
                    ClaimDecision::Blocked
                } else {
                    ClaimDecision::Provisional
                },
                evidence_refs: vec![experiment_ref.clone()],
                next_evidence_needed: vec![
                    "inspect per-trial failure reasons and artifacts".to_string(),
                ],
            },
            ClaimTraceEntry {
                claim: "adc-lab verified behavior across all fixed CPU frequencies.".to_string(),
                decision: ClaimDecision::Blocked,
                evidence_refs: Vec::new(),
                next_evidence_needed: vec![
                    "controlled operating point matrix with fixed frequency support".to_string(),
                ],
            },
            ClaimTraceEntry {
                claim: "experiment matrix proves production physical-footprint safety".to_string(),
                decision: ClaimDecision::Blocked,
                evidence_refs: Vec::new(),
                next_evidence_needed: vec![
                    "target-specific sustained thermal, wakeup, power, storage, flash, latency, and observer-effect evidence".to_string(),
                ],
            },
        ],
        time_unix_ms: now_unix_ms(),
    }
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

fn command_familiarize_read_only(args: FamiliarizeReadOnlyCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let started_at = now_unix_ms();
    let duration = parse_duration(&args.duration)?;
    let interval = parse_duration(&args.sample_interval)?;
    let signals = if args.signals.is_empty() {
        vec![Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory]
    } else {
        args.signals
    };

    persist_inventory(&run, &target)?;
    let (toolchain, _, toolchain_ref) = persist_toolchain(&run, &target)?;
    persist_toolchain_qualifications(&run, &toolchain, Some(toolchain_ref))?;
    persist_observation(&run, &target, duration, interval, signals)?;
    let (_, _, claim_trace_ref) = persist_read_only_claim_trace(&run, target.target_id.clone())?;
    let ended_at = now_unix_ms();
    let (_, _, run_manifest_ref) = persist_run_manifest(
        &run,
        target.target_id.clone(),
        target.raw.clone(),
        started_at,
        ended_at,
    )?;
    let (pack, _, familiarization_pack_ref) =
        persist_familiarization_pack(&run, target.target_id.clone())?;

    print_json(&ReadOnlyFamiliarizeOutput {
        run_id: run.run_id,
        target_id: target.target_id,
        run_manifest_ref,
        familiarization_pack_ref,
        claim_trace_ref,
        value: pack,
    })
}

fn command_report_pack(args: ReportPackCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    if !run
        .run_dir
        .join("reports/claim_evidence_trace.json")
        .exists()
    {
        persist_read_only_claim_trace(&run, args.target_id.clone())?;
    }
    let now = now_unix_ms();
    persist_run_manifest(&run, args.target_id.clone(), args.target, now, now)?;
    let (pack, path, _) = persist_familiarization_pack(&run, args.target_id)?;
    print_artifact(&run, &path, pack)
}

fn command_report_operating_point(args: ReportPackCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let coverage = operating_point_coverage(&run.run_dir, args.target_id.clone())?;
    let cost = capability_cost_model(&run.run_dir, args.target_id)?;
    let coverage_path = run.run_dir.join("reports/operating_point_coverage.json");
    let cost_path = run.run_dir.join("reports/capability_cost_model.json");
    let coverage_ref = write_json_artifact(&run, &coverage_path, &coverage)?;
    let cost_ref = write_json_artifact(&run, &cost_path, &cost)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: coverage.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.operating_point".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&coverage.coverage_status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: cost.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.capability_cost".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&cost.model_status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    print_json(&serde_json::json!({
        "operating_point_coverage_ref": coverage_ref,
        "capability_cost_model_ref": cost_ref,
        "coverage": coverage,
        "cost_model": cost
    }))
}

fn command_report_capability_profile(args: TargetCapabilityProfileCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let workload: WorkloadProfile = read_json(&args.workload).with_context(|| {
        format!(
            "failed to read workload profile {}",
            args.workload.display()
        )
    })?;
    let profile = target_capability_profile(&run.run_dir, args.target_id, workload)?;
    let safe_workload_id = safe_artifact_id(&profile.workload_id, "workload");
    let path = run
        .run_dir
        .join("reports")
        .join(format!("target_capability_profile.{safe_workload_id}.json"));
    let artifact_ref = write_json_artifact(&run, &path, &profile)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: profile.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.target_capability_profile".to_string(),
            operation_id: Some(profile.workload_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&profile.capability_status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    print_json(&serde_json::json!({
        "target_capability_profile_ref": artifact_ref,
        "profile": profile
    }))
}

fn command_health_check(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let output = build_health_output(&target);
    print_json(&output)
}

fn command_privilege_provider_status(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let status = privilege_provider_status(target.target_id.clone());
    let path = run.run_dir.join("privilege/privilege_provider_status.json");
    write_json_artifact(&run, &path, &status)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: status.target_id.clone(),
            actor: Actor::codex(),
            operation: "privilege.provider_status".to_string(),
            operation_id: Some(status.active_provider_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    print_artifact(&run, &path, status)
}

fn build_health_output(target: &TargetSpec) -> HealthOutput {
    let inventory_available = collect_inventory(target).is_ok();
    let toolchain_available = discover_toolchain(target).is_ok();
    HealthOutput {
        schema_version: "lab.health_check.v1".to_string(),
        target_id: target.target_id.clone(),
        status: if inventory_available && toolchain_available {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        inventory_available,
        toolchain_available,
    }
}

fn command_tool_qualify(args: ToolQualifyCommand) -> Result<()> {
    let run = create_or_open_run(args.run_dir.clone())?;
    let manifest: ToolManifest = read_yaml(&args.manifest)?;
    let pending_evidence = build_pending_tool_qualification_evidence(&run, &manifest, &args)?;
    let report = qualify_tool_with_evidence(
        manifest,
        pending_evidence
            .as_ref()
            .map(|pending| pending.evidence.clone()),
    )?;
    if let Some(pending) = pending_evidence.as_ref() {
        persist_pending_tool_qualification_evidence(&run, pending)?;
    }
    let path = run
        .run_dir
        .join("tools")
        .join(format!("{}.qualification.json", report.tool_id));
    write_json_artifact(&run, &path, &report)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: "toolchain".to_string(),
            actor: Actor::codex(),
            operation: "tool.qualify".to_string(),
            operation_id: Some(report.tool_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: if report.evidence_accepted {
                "qualified".to_string()
            } else {
                "recorded_unqualified".to_string()
            },
        },
    )?;
    print_artifact(&run, &path, report)
}

fn build_pending_tool_qualification_evidence(
    run: &RunContext,
    manifest: &ToolManifest,
    args: &ToolQualifyCommand,
) -> Result<Option<PendingToolQualificationEvidence>> {
    if !tool_qualification_evidence_requested(args) {
        return Ok(None);
    }

    let Some(tool_version) = args.tool_version.clone() else {
        anyhow::bail!("complete tool qualification evidence requires --tool-version");
    };
    let Some(tool_sha256) = args.tool_sha256.clone() else {
        anyhow::bail!("complete tool qualification evidence requires --tool-sha256");
    };
    let Some(output_schema_input) = args.output_schema.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --output-schema");
    };
    let Some(dry_run_input) = args.dry_run_output.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --dry-run-output");
    };
    let Some(manual_comparison_input) = args.manual_comparison.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --manual-comparison");
    };
    let Some(static_safety_review_input) = args.static_safety_review.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --static-safety-review");
    };

    let output_schema = read_json_evidence_file(
        output_schema_input,
        "output schema",
        AGENT_ADAPTER_OUTPUT_BYTES_MAX,
    )?
    .0;
    let (dry_run, validated_output_bytes) = read_json_evidence_file(
        dry_run_input,
        "dry-run output",
        manifest.bounded.output_bytes_max,
    )?;
    let manual_comparison = read_json_evidence_file(
        manual_comparison_input,
        "manual comparison",
        manifest.bounded.output_bytes_max,
    )?
    .0;
    let static_safety_review = read_text_evidence_file(
        static_safety_review_input,
        "static safety review",
        64 * 1024,
    )?;

    let safe_tool_id = safe_artifact_id(&manifest.tool_id, "TOOL");
    let output_schema_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.output_schema.json"));
    let dry_run_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.dry_run.json"));
    let manual_comparison_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.manual_comparison.json"));
    let static_safety_review_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.static_safety_review.txt"));

    let evidence = ToolQualificationEvidence {
        tool_version,
        tool_sha256,
        output_schema_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.output_schema.json"),
        ),
        dry_run_ref: planned_run_artifact_ref(run, &format!("tools/{safe_tool_id}.dry_run.json")),
        manual_comparison_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.manual_comparison.json"),
        ),
        static_safety_review_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.static_safety_review.txt"),
        ),
        validated_output_bytes,
    };

    Ok(Some(PendingToolQualificationEvidence {
        evidence,
        output_schema_path,
        output_schema,
        dry_run_path,
        dry_run,
        manual_comparison_path,
        manual_comparison,
        static_safety_review_path,
        static_safety_review,
    }))
}

fn tool_qualification_evidence_requested(args: &ToolQualifyCommand) -> bool {
    args.tool_version.is_some()
        || args.tool_sha256.is_some()
        || args.output_schema.is_some()
        || args.dry_run_output.is_some()
        || args.manual_comparison.is_some()
        || args.static_safety_review.is_some()
}

fn persist_pending_tool_qualification_evidence(
    run: &RunContext,
    pending: &PendingToolQualificationEvidence,
) -> Result<()> {
    write_json_artifact(run, &pending.output_schema_path, &pending.output_schema)?;
    write_json_artifact(run, &pending.dry_run_path, &pending.dry_run)?;
    write_json_artifact(
        run,
        &pending.manual_comparison_path,
        &pending.manual_comparison,
    )?;
    write_text_artifact(
        run,
        &pending.static_safety_review_path,
        &pending.static_safety_review,
    )?;
    Ok(())
}

fn planned_run_artifact_ref(run: &RunContext, relative_path: &str) -> String {
    format!("artifact://lab/runs/{}/{}", run.run_id, relative_path)
}

fn read_json_evidence_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<(serde_json::Value, u64)> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {label} evidence"))?;
    let len = bytes.len() as u64;
    if len == 0 {
        anyhow::bail!("{label} evidence is empty");
    }
    if len > max_bytes {
        anyhow::bail!("{label} evidence exceeds {max_bytes} byte bound");
    }
    let value = serde_json::from_slice(&bytes)
        .with_context(|| format!("{label} evidence must be valid JSON"))?;
    Ok((value, len))
}

fn read_text_evidence_file(path: &Path, label: &str, max_bytes: u64) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {label} evidence"))?;
    if bytes.is_empty() {
        anyhow::bail!("{label} evidence is empty");
    }
    if bytes.len() as u64 > max_bytes {
        anyhow::bail!("{label} evidence exceeds {max_bytes} byte bound");
    }
    let text =
        String::from_utf8(bytes).with_context(|| format!("{label} evidence must be UTF-8 text"))?;
    if text.trim().is_empty() {
        anyhow::bail!("{label} evidence is blank");
    }
    Ok(text)
}

fn command_tool_qualify_inventory(args: ToolQualifyInventoryCommand) -> Result<()> {
    let run = create_or_open_run(
        args.run_dir
            .or_else(|| infer_run_dir_from_artifact(&args.inventory)),
    )?;
    let inventory: ToolchainInventory = read_json(&args.inventory)?;
    let inventory_ref = run.artifact_uri(&args.inventory).ok();
    let (summary, path, _) = persist_toolchain_qualifications(&run, &inventory, inventory_ref)?;
    print_artifact(&run, &path, summary)
}

fn persist_inventory(
    run: &RunContext,
    target: &TargetSpec,
) -> Result<(TargetInventory, PathBuf, String)> {
    let inventory = collect_inventory(target)?;
    let path = run.run_dir.join("inventory/target_inventory.json");
    let artifact_ref = write_json_artifact(run, &path, &inventory)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: inventory.target_id.clone(),
            actor: Actor::codex(),
            operation: "inventory".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    Ok((inventory, path, artifact_ref))
}

fn persist_toolchain(
    run: &RunContext,
    target: &TargetSpec,
) -> Result<(ToolchainInventory, PathBuf, String)> {
    let inventory = discover_toolchain(target)?;
    let path = run.run_dir.join("toolchain/toolchain_inventory.json");
    let artifact_ref = write_json_artifact(run, &path, &inventory)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: inventory.target_id.clone(),
            actor: Actor::codex(),
            operation: "toolchain.discover".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    Ok((inventory, path, artifact_ref))
}

fn persist_toolchain_qualifications(
    run: &RunContext,
    inventory: &ToolchainInventory,
    inventory_ref: Option<String>,
) -> Result<(ToolQualificationSummary, PathBuf, String)> {
    let reports = qualify_toolchain_inventory(inventory, inventory_ref);
    let mut reports_with_refs = Vec::new();
    for report in reports {
        let file_name = format!(
            "{}.qualification.json",
            safe_artifact_id(&report.tool_id, "TOOL")
        );
        let path = run.run_dir.join("tools").join(file_name);
        let artifact_ref = write_json_artifact(run, &path, &report)?;
        reports_with_refs.push((report, artifact_ref));
    }
    let summary = summarize_tool_qualifications(inventory.target_id.clone(), &reports_with_refs);
    let path = run.run_dir.join("tools/tool_qualification_summary.json");
    let artifact_ref = write_json_artifact(run, &path, &summary)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: inventory.target_id.clone(),
            actor: Actor::codex(),
            operation: "tool.qualify_inventory".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    Ok((summary, path, artifact_ref))
}

fn persist_observation(
    run: &RunContext,
    target: &TargetSpec,
    duration: Duration,
    interval: Duration,
    signals: Vec<Signal>,
) -> Result<(ObservationResult, PathBuf, String)> {
    let observation = match target.transport {
        TargetTransport::Local => {
            observe_local(target.target_id.clone(), duration, interval, signals)?
        }
        TargetTransport::Ssh => observe_ssh(target, duration, interval, &signals)?,
    };
    let path = run.run_dir.join("observations/observe.json");
    let artifact_ref = write_json_artifact(run, &path, &observation)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: observation.target_id.clone(),
            actor: Actor::codex(),
            operation: "observe".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "collected".to_string(),
        },
    )?;
    Ok((observation, path, artifact_ref))
}

fn persist_read_only_claim_trace(
    run: &RunContext,
    target_id: String,
) -> Result<(ClaimEvidenceTrace, PathBuf, String)> {
    let trace = read_only_claim_trace(&run.run_dir, target_id.clone())?;
    let path = run.run_dir.join("reports/claim_evidence_trace.json");
    let artifact_ref = write_json_artifact(run, &path, &trace)?;
    append_audit_event(
        run,
        AuditInput {
            target_id,
            actor: Actor::codex(),
            operation: "report.claim_trace".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    Ok((trace, path, artifact_ref))
}

fn persist_run_manifest(
    run: &RunContext,
    target_id: String,
    target: String,
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
) -> Result<(RunManifest, PathBuf, String)> {
    append_audit_event(
        run,
        AuditInput {
            target_id: target_id.clone(),
            actor: Actor::codex(),
            operation: "run_manifest.write".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    let manifest = run_manifest(
        &run.run_dir,
        target_id,
        target,
        started_at_unix_ms,
        ended_at_unix_ms,
        env!("CARGO_PKG_VERSION").to_string(),
    )?;
    let path = run.run_dir.join("run_manifest.json");
    let artifact_ref = write_json_artifact(run, &path, &manifest)?;
    Ok((manifest, path, artifact_ref))
}

fn persist_familiarization_pack(
    run: &RunContext,
    target_id: String,
) -> Result<(FamiliarizationPack, PathBuf, String)> {
    append_audit_event(
        run,
        AuditInput {
            target_id: target_id.clone(),
            actor: Actor::codex(),
            operation: "report.pack".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    let pack = pack_run(&run.run_dir, target_id)?;
    let path = run.run_dir.join("reports/familiarization_pack.json");
    let artifact_ref = write_json_artifact(run, &path, &pack)?;
    Ok((pack, path, artifact_ref))
}

fn persist_control_result(
    run: &RunContext,
    result: &ControlResult,
    approval_ref: Option<String>,
) -> Result<()> {
    let result_path = run
        .run_dir
        .join("plans")
        .join(format!("{}.result.json", result.result_id));
    write_json_pretty(&result_path, result)?;
    let lease_ref = if let Some(lease) = &result.restore_lease {
        let lease_path = run
            .run_dir
            .join("leases")
            .join(format!("{}.json", lease.lease_id));
        write_json_pretty(&lease_path, lease)?;
        Some(run.artifact_uri(&lease_path)?)
    } else {
        None
    };
    append_audit_event(
        run,
        AuditInput {
            target_id: result.target_id.clone(),
            actor: Actor::codex(),
            operation: match result.status {
                ControlResultStatus::Restored => "restore".to_string(),
                _ => "control.apply".to_string(),
            },
            operation_id: Some(result.operation_id.clone()),
            risk_tier: result.risk_tier.clone(),
            approval_ref,
            restore_lease_ref: lease_ref,
            result: format!("{:?}", result.status).to_lowercase(),
        },
    )?;
    Ok(())
}

fn persist_approval_record(run: &RunContext, approval: &ApprovalRecord) -> Result<String> {
    let (_, artifact_ref) = write_approval_record(run, approval)?;
    Ok(artifact_ref)
}

fn write_approval_record(run: &RunContext, approval: &ApprovalRecord) -> Result<(PathBuf, String)> {
    let file_name = format!(
        "{}.json",
        safe_artifact_id(&approval.approval_id, "APPROVAL")
    );
    let path = run.run_dir.join("approvals").join(file_name);
    write_json_pretty(&path, approval)?;
    let artifact_ref = run.artifact_uri(&path)?;
    Ok((path, artifact_ref))
}

fn persist_restore_health_check(run: &RunContext) -> Result<()> {
    let target = TargetSpec::parse("local")?;
    let output = build_health_output(&target);
    let path = run.run_dir.join("health/restore_health_check.json");
    write_json_artifact(run, &path, &output)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: output.target_id.clone(),
            actor: Actor::codex(),
            operation: "health-check.restore".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: output.status,
        },
    )?;
    Ok(())
}

fn safe_artifact_id(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

fn infer_run_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some("plans")
        || parent.file_name().and_then(|name| name.to_str()) == Some("leases")
    {
        return parent.parent().map(Path::to_path_buf);
    }
    None
}

fn infer_run_dir_from_artifact(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    match parent.file_name().and_then(|name| name.to_str()) {
        Some("inventory" | "toolchain" | "observations" | "reports" | "tools") => {
            parent.parent().map(Path::to_path_buf)
        }
        _ => None,
    }
}

fn invoke_helper_apply(plan: &Path, approval: Option<&Path>) -> Result<ControlResult> {
    let helper = Path::new(DEFAULT_PRIV_HELPER);
    validate_priv_helper_path(helper)?;
    let mut command = Command::new("sudo");
    command.arg(helper).arg("apply").arg("--plan").arg(plan);
    if let Some(approval) = approval {
        command.arg("--approval").arg(approval);
    }
    let output = command.output().context("failed to invoke sudo helper")?;
    if !output.status.success() {
        anyhow::bail!(
            "helper apply failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn invoke_helper_restore(lease: &Path) -> Result<ControlResult> {
    let helper = Path::new(DEFAULT_PRIV_HELPER);
    validate_priv_helper_path(helper)?;
    let output = Command::new("sudo")
        .arg(helper)
        .arg("restore")
        .arg("--lease")
        .arg(lease)
        .output()
        .context("failed to invoke sudo helper")?;
    if !output.status.success() {
        anyhow::bail!(
            "helper restore failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn observe_ssh(
    target: &TargetSpec,
    duration: Duration,
    interval: Duration,
    signals: &[Signal],
) -> Result<ObservationResult> {
    let signal_arg = signals
        .iter()
        .map(|signal| match signal {
            Signal::Cpu => "cpu",
            Signal::Freq => "freq",
            Signal::Thermal => "thermal",
            Signal::Memory => "memory",
        })
        .collect::<Vec<_>>()
        .join(",");
    let output = Command::new("ssh")
        .arg(&target.endpoint)
        .arg(ssh_runner_program()?)
        .arg("observe")
        .arg("--duration")
        .arg(format!("{}s", duration.as_secs().max(1)))
        .arg("--sample-interval")
        .arg(format!("{}s", interval.as_secs().max(1)))
        .arg("--signals")
        .arg(signal_arg)
        .arg("--json")
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "ssh observe failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut result: ObservationResult = serde_json::from_slice(&output.stdout)?;
    result.target_id = target.target_id.clone();
    Ok(result)
}

fn load_cpu_ssh(
    target: &TargetSpec,
    workers: usize,
    duration: Duration,
    abort_temp_c: Option<f64>,
    operator_abort_file: Option<&Path>,
) -> Result<LoadResult> {
    let mut command = Command::new("ssh");
    command
        .arg(&target.endpoint)
        .arg(ssh_runner_program()?)
        .arg("load")
        .arg("cpu")
        .arg("--workers")
        .arg(workers.to_string())
        .arg("--duration")
        .arg(format!("{}s", duration.as_secs().max(1)))
        .arg("--json");
    if let Some(limit) = abort_temp_c {
        command.arg("--abort-temp-c").arg(limit.to_string());
    }
    if let Some(path) = operator_abort_file {
        command.arg("--operator-abort-file").arg(path);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "ssh load cpu failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut result: LoadResult = serde_json::from_slice(&output.stdout)?;
    result.target_id = target.target_id.clone();
    Ok(result)
}

fn existing_run_context(run_dir: PathBuf) -> RunContext {
    let run_id = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("LAB-RUN-unknown")
        .to_string();
    RunContext { run_id, run_dir }
}

fn write_json_artifact<T: Serialize>(run: &RunContext, path: &Path, value: &T) -> Result<String> {
    write_json_pretty(path, value)?;
    Ok(run.artifact_uri(path)?)
}

fn write_text_artifact(run: &RunContext, path: &Path, value: &str) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create artifact directory {}", parent.display()))?;
    }
    fs::write(path, value)
        .with_context(|| format!("failed to write artifact {}", path.display()))?;
    Ok(run.artifact_uri(path)?)
}

fn print_artifact<T: Serialize>(run: &RunContext, path: &Path, value: T) -> Result<()> {
    let artifact_ref = run.artifact_uri(path)?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value,
    })
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
