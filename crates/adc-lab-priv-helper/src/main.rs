use adc_lab_core::*;
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "adc-lab-priv-helper")]
#[command(about = "Root-owned typed helper for adc-lab privileged operations")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Apply(ApplyArgs),
    Restore(RestoreArgs),
}

#[derive(Debug, Args)]
struct ApplyArgs {
    #[arg(long)]
    plan: PathBuf,
    #[arg(long)]
    approval: Option<PathBuf>,
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct RestoreArgs {
    #[arg(long)]
    lease: PathBuf,
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    if print_version_if_requested("adc-lab-priv-helper")? {
        return Ok(());
    }
    let cli = Cli::parse();
    match cli.command {
        Commands::Apply(args) => {
            let plan: ControlPlan = read_json(&args.plan)?;
            let approval = args.approval.as_ref().map(read_json).transpose()?;
            let result = apply_control_plan(
                &plan,
                approval.as_ref(),
                &LinuxCpufreqBackend::default(),
                args.dry_run,
            );
            print_json(&result)
        }
        Commands::Restore(args) => {
            let lease: RestoreLease = read_json(&args.lease)?;
            let result = restore_lease(&lease, &LinuxCpufreqBackend::default(), args.dry_run);
            print_json(&result)
        }
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
