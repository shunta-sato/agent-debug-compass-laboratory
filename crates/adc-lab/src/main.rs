use adc_lab_core::*;
use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

mod commands;

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
    Pressure {
        #[command(subcommand)]
        command: PressureCommand,
    },
    Workload {
        #[command(subcommand)]
        command: WorkloadCommand,
    },
    Decide {
        #[command(subcommand)]
        command: DecideCommand,
    },
    Constraints {
        #[command(subcommand)]
        command: ConstraintsCommand,
    },
    #[command(name = "workload-fixture")]
    WorkloadFixture {
        #[command(subcommand)]
        command: WorkloadFixtureCommand,
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
    Doctor(TargetCommand),
    #[command(name = "install-plan")]
    InstallPlan(PrivilegeInstallPlanCommand),
    #[command(name = "uninstall-plan")]
    UninstallPlan(TargetCommand),
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

#[derive(Debug, Subcommand)]
enum PressureCommand {
    Run(PressureRunCommand),
    Composite(PressureCompositeCommand),
}

#[derive(Debug, Subcommand)]
enum WorkloadCommand {
    Run(WorkloadRunCommand),
}

#[derive(Debug, Args)]
struct WorkloadRunCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    target_id: Option<String>,
    #[arg(long, value_enum, default_value_t = WorkloadExecutionModeArg::Local)]
    execution_mode: WorkloadExecutionModeArg,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum WorkloadExecutionModeArg {
    Local,
    TargetLocal,
}

impl From<WorkloadExecutionModeArg> for WorkloadExecutionMode {
    fn from(value: WorkloadExecutionModeArg) -> Self {
        match value {
            WorkloadExecutionModeArg::Local => WorkloadExecutionMode::Local,
            WorkloadExecutionModeArg::TargetLocal => WorkloadExecutionMode::TargetLocal,
        }
    }
}

#[derive(Debug, Subcommand)]
enum DecideCommand {
    Suitability(SuitabilityCommand),
}

#[derive(Debug, Args)]
struct SuitabilityCommand {
    #[arg(long)]
    target_run: PathBuf,
    #[arg(long)]
    target_contract: PathBuf,
    #[arg(long)]
    workload_demand: PathBuf,
    #[arg(long)]
    policy: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum ConstraintsCommand {
    Generate(ConstraintsGenerateCommand),
    Check(ConstraintsCheckCommand),
}

#[derive(Debug, Args)]
struct ConstraintsGenerateCommand {
    #[arg(long)]
    decision: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long = "agent-instructions-out")]
    agent_instructions_out: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ConstraintsCheckCommand {
    #[arg(long)]
    constraints: PathBuf,
    #[arg(long)]
    path: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum WorkloadFixtureCommand {
    #[command(name = "bounded-smoke")]
    BoundedSmoke(BoundedSmokeCommand),
}

#[derive(Debug, Args)]
struct BoundedSmokeCommand {
    #[arg(long, default_value_t = 500)]
    duration_ms: u64,
    #[arg(long, default_value_t = 1024 * 1024)]
    memory_bytes: u64,
    #[arg(long, default_value_t = 64 * 1024)]
    storage_bytes: u64,
    #[arg(long)]
    storage_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct PressureRunCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    kind: ResourcePressureKind,
    #[arg(long, default_value = "1s")]
    duration: String,
    #[arg(long, default_value_t = 1)]
    workers: usize,
    #[arg(long)]
    abort_temp_c: Option<f64>,
    #[arg(long, default_value_t = 8 * 1024 * 1024)]
    memory_bytes: u64,
    #[arg(long, default_value_t = 1024 * 1024)]
    storage_bytes: u64,
    #[arg(long, default_value_t = 0)]
    network_bytes: u64,
    #[arg(long)]
    network_endpoint: Option<String>,
    #[arg(long)]
    storage_dir: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PressureCompositeCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    scenario: CompositeBoundaryScenario,
    #[arg(long, default_value = "1s")]
    duration: String,
    #[arg(long, default_value_t = 1)]
    workers: usize,
    #[arg(long)]
    abort_temp_c: Option<f64>,
    #[arg(long, default_value_t = 8 * 1024 * 1024)]
    memory_bytes: u64,
    #[arg(long, default_value_t = 1024 * 1024)]
    storage_bytes: u64,
    #[arg(long, default_value_t = 0)]
    network_bytes: u64,
    #[arg(long)]
    network_endpoint: Option<String>,
    #[arg(long)]
    storage_dir: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    json: bool,
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
    OperatingContract(OperatingContractCommand),
    ValidateRun(ValidateRunCommand),
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
struct OperatingContractCommand {
    #[arg(long)]
    run: PathBuf,
    #[arg(long = "include-run")]
    include_runs: Vec<PathBuf>,
    #[arg(long, default_value = "unknown-target")]
    target_id: String,
    #[arg(long, default_value = "unknown-target-class")]
    target_class: String,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ValidateRunCommand {
    #[arg(long)]
    run: PathBuf,
    #[arg(long, default_value = "target-operating-contract-fullset")]
    profile: String,
    #[arg(long = "expected-governors", value_delimiter = ',')]
    expected_governors: Vec<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long = "gaps-out")]
    gaps_out: Option<PathBuf>,
    #[arg(long)]
    allow_non_measured: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PrivilegeInstallPlanCommand {
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long)]
    helper_bin: Option<PathBuf>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
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

fn main() -> Result<()> {
    if print_version_if_requested("adc-lab")? {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Inventory(args) => commands::target::command_inventory(args),
        Commands::Toolchain { command } => match command {
            ToolchainCommand::Discover(args) => commands::target::command_toolchain_discover(args),
        },
        Commands::Observe(args) => commands::target::command_observe(args),
        Commands::Control { command } => match command {
            ControlCommand::Plan(args) => commands::control::command_control_plan(args),
            ControlCommand::Approve(args) => commands::control::command_control_approve(args),
            ControlCommand::Apply(args) => commands::control::command_control_apply(args),
        },
        Commands::Restore(args) => commands::control::command_restore(args),
        Commands::Load { command } => match command {
            LoadCommand::Cpu(args) => commands::load::command_load_cpu(args),
        },
        Commands::Pressure { command } => match command {
            PressureCommand::Run(args) => commands::pressure::command_pressure_run(args),
            PressureCommand::Composite(args) => {
                commands::pressure::command_pressure_composite(args)
            }
        },
        Commands::Workload { command } => match command {
            WorkloadCommand::Run(args) => commands::workload::command_workload_run(args),
        },
        Commands::Decide { command } => match command {
            DecideCommand::Suitability(args) => commands::decide::command_decide_suitability(args),
        },
        Commands::Constraints { command } => match command {
            ConstraintsCommand::Generate(args) => {
                commands::constraints::command_constraints_generate(args)
            }
            ConstraintsCommand::Check(args) => {
                commands::constraints::command_constraints_check(args)
            }
        },
        Commands::WorkloadFixture { command } => match command {
            WorkloadFixtureCommand::BoundedSmoke(args) => {
                commands::workload::command_workload_fixture_bounded_smoke(args)
            }
        },
        Commands::Experiment { command } => match command {
            ExperimentCommand::Run(args) => commands::experiment::command_experiment_run(args),
        },
        Commands::Familiarize { command } => match command {
            FamiliarizeCommand::ReadOnly(args) => {
                commands::familiarize::command_familiarize_read_only(args)
            }
        },
        Commands::Report { command } => match command {
            ReportCommand::Pack(args) => commands::report::command_report_pack(args),
            ReportCommand::OperatingPoint(args) => {
                commands::report::command_report_operating_point(args)
            }
            ReportCommand::OperatingContract(args) => {
                commands::report::command_report_operating_contract(args)
            }
            ReportCommand::ValidateRun(args) => commands::report::command_report_validate_run(args),
        },
        Commands::HealthCheck(args) => commands::target::command_health_check(args),
        Commands::Privilege { command } => match command {
            PrivilegeCommand::ProviderStatus(args) => {
                commands::privilege::command_privilege_provider_status(args)
            }
            PrivilegeCommand::Doctor(args) => commands::privilege::command_privilege_doctor(args),
            PrivilegeCommand::InstallPlan(args) => {
                commands::privilege::command_privilege_install_plan(args)
            }
            PrivilegeCommand::UninstallPlan(args) => {
                commands::privilege::command_privilege_uninstall_plan(args)
            }
        },
        Commands::Tool { command } => match command {
            ToolCommand::Qualify(args) => commands::tool::command_tool_qualify(args),
            ToolCommand::QualifyInventory(args) => {
                commands::tool::command_tool_qualify_inventory(args)
            }
        },
    }
}

fn print_version_if_requested(name: &str) -> Result<bool> {
    let Some(arg) = std::env::args_os().nth(1) else {
        return Ok(false);
    };
    if arg == "--version" || arg == "-V" {
        commands::common::print_json(&build_info(name))?;
        Ok(true)
    } else {
        Ok(false)
    }
}
