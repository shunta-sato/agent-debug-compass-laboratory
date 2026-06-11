use super::super::*;
use super::common::*;
use adc_lab_core::ids::now_unix_ms;
use serde::Serialize;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadOnlyFamiliarizeOutput {
    run_id: String,
    target_id: String,
    run_manifest_ref: String,
    run_report_ref: String,
    value: Artifact<RunReportPayload>,
}

pub(crate) fn command_familiarize_read_only(args: FamiliarizeReadOnlyCommand) -> Result<()> {
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
    let (run_report, _, run_report_ref) = persist_run_report(&run, target.target_id.clone())?;
    let ended_at = now_unix_ms();
    let (_, _, run_manifest_ref) = persist_run_manifest(
        &run,
        target.target_id.clone(),
        target.raw.clone(),
        started_at,
        ended_at,
    )?;

    print_json(&ReadOnlyFamiliarizeOutput {
        run_id: run.run_id,
        target_id: target.target_id,
        run_manifest_ref,
        run_report_ref,
        value: run_report,
    })
}

fn persist_inventory(
    run: &RunContext,
    target: &TargetSpec,
) -> Result<(TargetInventory, PathBuf, String)> {
    persist_target_runner_version_if_absent(run, target)?;
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
    persist_target_runner_version_if_absent(run, target)?;
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
    persist_target_runner_version_if_absent(run, target)?;
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
