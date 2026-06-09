use adc_lab_core::*;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;

#[derive(Debug, Parser)]
#[command(name = "adc-lab-target")]
#[command(about = "Non-root fixed-command target runner for adc-lab")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Inventory(JsonFlag),
    Observe(ObserveArgs),
    Load {
        #[command(subcommand)]
        command: LoadCommand,
    },
    HealthCheck(JsonFlag),
}

#[derive(Debug, Args)]
struct JsonFlag {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ObserveArgs {
    #[arg(long)]
    duration: String,
    #[arg(long, default_value = "1s")]
    sample_interval: String,
    #[arg(long, value_delimiter = ',')]
    signals: Vec<Signal>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum LoadCommand {
    Cpu(LoadCpuArgs),
}

#[derive(Debug, Args)]
struct LoadCpuArgs {
    #[arg(long, default_value_t = 1)]
    workers: usize,
    #[arg(long)]
    duration: String,
    #[arg(long)]
    abort_temp_c: Option<f64>,
    #[arg(long)]
    operator_abort_file: Option<std::path::PathBuf>,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct HealthOutput {
    schema_version: String,
    target_id: String,
    status: String,
}

fn main() -> Result<()> {
    if print_version_if_requested("adc-lab-target")? {
        return Ok(());
    }
    let cli = Cli::parse();
    let target = TargetSpec::parse("local")?;
    match cli.command {
        Commands::Inventory(_) => print_json(&collect_inventory(&target)?),
        Commands::Observe(args) => {
            let duration = parse_duration(&args.duration)?;
            let interval = parse_duration(&args.sample_interval)?;
            let signals = if args.signals.is_empty() {
                vec![Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory]
            } else {
                args.signals
            };
            print_json(&observe_local(
                target.target_id,
                duration,
                interval,
                signals,
            )?)
        }
        Commands::Load { command } => match command {
            LoadCommand::Cpu(args) => {
                let duration = parse_duration(&args.duration)?;
                let plan = new_cpu_load_plan_with_operator_abort(
                    target.target_id,
                    args.workers,
                    duration,
                    args.abort_temp_c,
                    args.operator_abort_file.is_some(),
                )?;
                print_json(&run_cpu_load_with_options(
                    &plan,
                    &CpuLoadRuntimeOptions {
                        operator_abort_file: args.operator_abort_file,
                    },
                )?)
            }
        },
        Commands::HealthCheck(_) => print_json(&HealthOutput {
            schema_version: "lab.health_check.v1".to_string(),
            target_id: target.target_id,
            status: "ok".to_string(),
        }),
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

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
