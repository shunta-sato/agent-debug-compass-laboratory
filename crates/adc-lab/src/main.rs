use adc_lab_core::ids::now_unix_ms;
use adc_lab_core::*;
use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
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
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ReportCommand {
    Pack(ReportPackCommand),
    OperatingPoint(ReportPackCommand),
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

#[derive(Debug, Subcommand)]
enum ToolCommand {
    Qualify(ToolQualifyCommand),
}

#[derive(Debug, Args)]
struct ToolQualifyCommand {
    #[arg(long)]
    manifest: PathBuf,
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
    let cli = Cli::parse();
    match cli.command {
        Commands::Inventory(args) => command_inventory(args),
        Commands::Toolchain { command } => match command {
            ToolchainCommand::Discover(args) => command_toolchain_discover(args),
        },
        Commands::Observe(args) => command_observe(args),
        Commands::Control { command } => match command {
            ControlCommand::Plan(args) => command_control_plan(args),
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
        },
        Commands::HealthCheck(args) => command_health_check(args),
        Commands::Tool { command } => match command {
            ToolCommand::Qualify(args) => command_tool_qualify(args),
        },
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
    print_json(&result)
}

fn command_load_cpu(args: LoadCpuCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let duration = parse_duration(&args.duration)?;
    let result = match target.transport {
        TargetTransport::Local => {
            let plan = new_cpu_load_plan(
                target.target_id.clone(),
                args.workers,
                duration,
                args.abort_temp_c,
            )?;
            let plan_path = run
                .run_dir
                .join("loads")
                .join(format!("{}.plan.json", plan.load_id));
            write_json_artifact(&run, &plan_path, &plan)?;
            run_cpu_load(&plan)?
        }
        TargetTransport::Ssh => load_cpu_ssh(&target, args.workers, duration, args.abort_temp_c)?,
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
    let (experiment_run, claim_trace) = run_experiment_matrix(
        &matrix,
        run.run_id.clone(),
        target.target_id.clone(),
        args.dry_run,
    );
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
            result: if args.dry_run {
                "planned"
            } else {
                "not_implemented"
            }
            .to_string(),
        },
    )?;
    print_artifact(&run, &run_path, experiment_run)
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
    persist_toolchain(&run, &target)?;
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
    let run_id = run
        .run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("LAB-RUN-unknown")
        .to_string();
    let coverage = operating_point_coverage(run_id.clone(), args.target_id.clone());
    let cost = capability_cost_model(run_id, args.target_id);
    let coverage_path = run.run_dir.join("reports/operating_point_coverage.json");
    let cost_path = run.run_dir.join("reports/capability_cost_model.json");
    let coverage_ref = write_json_artifact(&run, &coverage_path, &coverage)?;
    let cost_ref = write_json_artifact(&run, &cost_path, &cost)?;
    print_json(&serde_json::json!({
        "operating_point_coverage_ref": coverage_ref,
        "capability_cost_model_ref": cost_ref,
        "coverage": coverage,
        "cost_model": cost
    }))
}

fn command_health_check(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let inventory_available = collect_inventory(&target).is_ok();
    let toolchain_available = discover_toolchain(&target).is_ok();
    let output = HealthOutput {
        schema_version: "lab.health_check.v1".to_string(),
        target_id: target.target_id,
        status: if inventory_available && toolchain_available {
            "ok".to_string()
        } else {
            "degraded".to_string()
        },
        inventory_available,
        toolchain_available,
    };
    print_json(&output)
}

fn command_tool_qualify(args: ToolQualifyCommand) -> Result<()> {
    let run = create_or_open_run(args.run_dir)?;
    let manifest: ToolManifest = read_yaml(&args.manifest)?;
    let report = qualify_tool(manifest)?;
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
            result: "recorded_unqualified".to_string(),
        },
    )?;
    print_artifact(&run, &path, report)
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
    let file_name = format!(
        "{}.json",
        safe_artifact_id(&approval.approval_id, "APPROVAL")
    );
    let path = run.run_dir.join("approvals").join(file_name);
    write_json_pretty(&path, approval)?;
    Ok(run.artifact_uri(&path)?)
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
