use super::super::*;
use super::common::*;

pub(crate) fn command_inventory(args: TargetCommand) -> Result<()> {
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

pub(crate) fn command_toolchain_discover(args: TargetCommand) -> Result<()> {
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

pub(crate) fn command_observe(args: ObserveCommand) -> Result<()> {
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
    let mut store = evidence_store_for_run(&run)?;
    write_observation_artifact_v2(&mut store, &run.run_dir, observation.clone())?;
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

pub(crate) fn command_health_check(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let output = build_health_output(&target);
    print_json(&output)
}
