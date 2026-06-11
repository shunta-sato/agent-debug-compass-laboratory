use super::super::*;
use super::common::*;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;

pub(crate) fn command_pressure_run(args: PressureRunCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    persist_target_runner_version_if_absent(&run, &target)?;
    let options = PressureProbeOptions {
        duration: parse_duration(&args.duration)?,
        workers: args.workers,
        abort_temp_c: args.abort_temp_c,
        memory_bytes: args.memory_bytes,
        storage_bytes: args.storage_bytes,
        network_bytes: args.network_bytes,
        network_endpoint: args.network_endpoint,
        storage_dir: args.storage_dir,
    };
    let result = match target.transport {
        TargetTransport::Local => {
            run_resource_pressure(target.target_id.clone(), args.kind.clone(), &options)?
        }
        TargetTransport::Ssh => pressure_ssh(&target, args.kind.clone(), &options)?,
    };
    let operation_id = result.pressure_kind.as_str().to_string();
    let target_id = result.target_id.clone();
    let result_status = serde_json::to_string(&result.status)
        .unwrap_or_else(|_| "unknown".to_string())
        .trim_matches('"')
        .to_string();
    let relative = PathBuf::from("pressure").join(format!(
        "{}.{}.v2.json",
        result.pressure_kind.as_str(),
        safe_artifact_id(&result.result_id, "PRESSURE")
    ));
    let artifact = pressure_artifact_v2(run.run_id.clone(), result);
    let mut store = evidence_store_for_run(&run)?;
    let artifact_ref = store.write(&run.run_dir, &relative, &artifact)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id,
            actor: Actor::codex(),
            operation: "pressure.run".to_string(),
            operation_id: Some(operation_id),
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: result_status,
        },
    )?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value: artifact,
    })
}

pub(crate) fn command_pressure_composite(args: PressureCompositeCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    persist_target_runner_version_if_absent(&run, &target)?;
    let options = PressureProbeOptions {
        duration: parse_duration(&args.duration)?,
        workers: args.workers,
        abort_temp_c: args.abort_temp_c,
        memory_bytes: args.memory_bytes,
        storage_bytes: args.storage_bytes,
        network_bytes: args.network_bytes,
        network_endpoint: args.network_endpoint,
        storage_dir: args.storage_dir,
    };
    let result = match target.transport {
        TargetTransport::Local => {
            run_composite_boundary(target.target_id.clone(), args.scenario.clone(), &options)?
        }
        TargetTransport::Ssh => composite_ssh(&target, args.scenario.clone(), &options)?,
    };
    let operation_id = result.scenario.as_str().to_string();
    let target_id = result.target_id.clone();
    let result_status = serde_json::to_string(&result.status)
        .unwrap_or_else(|_| "unknown".to_string())
        .trim_matches('"')
        .to_string();
    let relative = PathBuf::from("composite").join(format!(
        "{}.{}.v2.json",
        result.scenario.as_str(),
        safe_artifact_id(&result.result_id, "COMPOSITE")
    ));
    let artifact = composite_artifact_v2(run.run_id.clone(), result);
    let mut store = evidence_store_for_run(&run)?;
    let artifact_ref = store.write(&run.run_dir, &relative, &artifact)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id,
            actor: Actor::codex(),
            operation: "pressure.composite".to_string(),
            operation_id: Some(operation_id),
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: result_status,
        },
    )?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value: artifact,
    })
}

fn pressure_ssh(
    target: &TargetSpec,
    kind: ResourcePressureKind,
    options: &PressureProbeOptions,
) -> Result<ResourcePressureResult> {
    let mut remote_args = vec![
        ssh_runner_program()?,
        "pressure".to_string(),
        "run".to_string(),
        "--kind".to_string(),
        kind.as_str().to_string(),
        "--duration".to_string(),
        format!("{}s", options.duration.as_secs().max(1)),
        "--workers".to_string(),
        options.workers.to_string(),
        "--memory-bytes".to_string(),
        options.memory_bytes.to_string(),
        "--storage-bytes".to_string(),
        options.storage_bytes.to_string(),
        "--network-bytes".to_string(),
        options.network_bytes.to_string(),
        "--json".to_string(),
    ];
    let mut command = Command::new("ssh");
    command.arg(&target.endpoint);
    if let Some(limit) = options.abort_temp_c {
        remote_args.push("--abort-temp-c".to_string());
        remote_args.push(limit.to_string());
    }
    if let Some(endpoint) = options.network_endpoint.as_ref() {
        remote_args.push("--network-endpoint".to_string());
        remote_args.push(endpoint.clone());
    }
    append_ssh_remote_args(&mut command, remote_args)?;
    if let Some(path) = options.storage_dir.as_ref() {
        command.arg(remote_shell_quote(OsStr::new("--storage-dir"))?);
        command.arg(remote_shell_quote(path.as_os_str())?);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "ssh pressure run failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut result: ResourcePressureResult = serde_json::from_slice(&output.stdout)?;
    result.target_id = target.target_id.clone();
    Ok(result)
}

fn composite_ssh(
    target: &TargetSpec,
    scenario: CompositeBoundaryScenario,
    options: &PressureProbeOptions,
) -> Result<CompositeBoundaryResult> {
    let mut remote_args = vec![
        ssh_runner_program()?,
        "pressure".to_string(),
        "composite".to_string(),
        "--scenario".to_string(),
        scenario.as_str().to_string(),
        "--duration".to_string(),
        format!("{}s", options.duration.as_secs().max(1)),
        "--workers".to_string(),
        options.workers.to_string(),
        "--memory-bytes".to_string(),
        options.memory_bytes.to_string(),
        "--storage-bytes".to_string(),
        options.storage_bytes.to_string(),
        "--network-bytes".to_string(),
        options.network_bytes.to_string(),
        "--json".to_string(),
    ];
    let mut command = Command::new("ssh");
    command.arg(&target.endpoint);
    if let Some(limit) = options.abort_temp_c {
        remote_args.push("--abort-temp-c".to_string());
        remote_args.push(limit.to_string());
    }
    if let Some(endpoint) = options.network_endpoint.as_ref() {
        remote_args.push("--network-endpoint".to_string());
        remote_args.push(endpoint.clone());
    }
    append_ssh_remote_args(&mut command, remote_args)?;
    if let Some(path) = options.storage_dir.as_ref() {
        command.arg(remote_shell_quote(OsStr::new("--storage-dir"))?);
        command.arg(remote_shell_quote(path.as_os_str())?);
    }
    let output = command.output()?;
    if !output.status.success() {
        anyhow::bail!(
            "ssh pressure composite failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let mut result: CompositeBoundaryResult = serde_json::from_slice(&output.stdout)?;
    result.target_id = target.target_id.clone();
    Ok(result)
}
