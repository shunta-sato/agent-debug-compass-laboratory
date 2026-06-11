use super::super::*;
use anyhow::Context;
use serde::Serialize;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ArtifactOutput<T: Serialize> {
    pub artifact_ref: String,
    pub value: T,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HealthOutput {
    pub schema_version: String,
    pub target_id: String,
    pub status: String,
    pub inventory_available: bool,
    pub toolchain_available: bool,
}

pub(super) fn build_health_output(target: &TargetSpec) -> HealthOutput {
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

pub(super) fn persist_toolchain_qualifications(
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

pub(super) fn persist_run_report(
    run: &RunContext,
    target_id: String,
) -> Result<(Artifact<RunReportPayload>, PathBuf, String)> {
    let mut store = evidence_store_for_run(run)?;
    let report = evaluate_run_report_v2(&store, &run.run_dir, target_id.clone())?;
    let relative_path = Path::new(RUN_REPORT_RELATIVE_PATH);
    let artifact_ref = store.write(&run.run_dir, relative_path, &report)?;
    let path = run.run_dir.join(relative_path);
    append_audit_event(
        run,
        AuditInput {
            target_id,
            actor: Actor::codex(),
            operation: "report.run".to_string(),
            operation_id: Some(report.id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&report.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    Ok((report, path, artifact_ref))
}

pub(super) fn persist_run_manifest(
    run: &RunContext,
    target_id: String,
    target: String,
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
) -> Result<(RunManifest, PathBuf, String)> {
    persist_controller_version_if_absent(run)?;
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
        build_info("adc-lab"),
    )?;
    let path = run.run_dir.join("run_manifest.json");
    let artifact_ref = write_json_artifact(run, &path, &manifest)?;
    Ok((manifest, path, artifact_ref))
}

pub(super) fn persist_controller_version_if_absent(run: &RunContext) -> Result<()> {
    let path = run.run_dir.join("tools/adc-lab.version.json");
    if path.exists() {
        return Ok(());
    }
    write_json_artifact(run, &path, &build_info("adc-lab"))?;
    append_audit_event(
        run,
        AuditInput {
            target_id: "controller".to_string(),
            actor: Actor::codex(),
            operation: "tool.version".to_string(),
            operation_id: Some("adc-lab".to_string()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    Ok(())
}

pub(super) fn persist_target_runner_version_if_absent(
    run: &RunContext,
    target: &TargetSpec,
) -> Result<()> {
    let path = run.run_dir.join("tools/adc-lab-target.version.json");
    if path.exists() {
        return Ok(());
    }
    let info = target_runner_build_info(target)?;
    write_json_artifact(run, &path, &info)?;
    append_audit_event(
        run,
        AuditInput {
            target_id: target.target_id.clone(),
            actor: Actor::codex(),
            operation: "tool.version".to_string(),
            operation_id: Some("adc-lab-target".to_string()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "recorded".to_string(),
        },
    )?;
    Ok(())
}

pub(super) fn safe_artifact_id(value: &str, fallback: &str) -> String {
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

pub(super) fn infer_run_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some("plans")
        || parent.file_name().and_then(|name| name.to_str()) == Some("leases")
    {
        return parent.parent().map(Path::to_path_buf);
    }
    None
}

pub(super) fn observe_ssh(
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
    let remote_args = vec![
        ssh_runner_program()?,
        "observe".to_string(),
        "--duration".to_string(),
        format!("{}s", duration.as_secs().max(1)),
        "--sample-interval".to_string(),
        format!("{}s", interval.as_secs().max(1)),
        "--signals".to_string(),
        signal_arg,
        "--json".to_string(),
    ];
    let mut command = Command::new("ssh");
    command.arg(&target.endpoint);
    append_ssh_remote_args(&mut command, remote_args)?;
    let output = command.output()?;
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

pub(super) fn load_cpu_ssh(
    target: &TargetSpec,
    workers: usize,
    duration: Duration,
    abort_temp_c: Option<f64>,
    operator_abort_file: Option<&Path>,
) -> Result<LoadResult> {
    let mut remote_args = vec![
        ssh_runner_program()?,
        "load".to_string(),
        "cpu".to_string(),
        "--workers".to_string(),
        workers.to_string(),
        "--duration".to_string(),
        format!("{}s", duration.as_secs().max(1)),
        "--json".to_string(),
    ];
    if let Some(limit) = abort_temp_c {
        remote_args.push("--abort-temp-c".to_string());
        remote_args.push(limit.to_string());
    }

    let mut command = Command::new("ssh");
    command.arg(&target.endpoint);
    append_ssh_remote_args(&mut command, remote_args)?;
    if let Some(path) = operator_abort_file {
        command.arg(remote_shell_quote(OsStr::new("--operator-abort-file"))?);
        command.arg(remote_shell_quote(path.as_os_str())?);
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

pub(super) fn append_ssh_remote_args<I, S>(command: &mut Command, args: I) -> Result<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    for arg in args {
        command.arg(remote_shell_quote(arg.as_ref())?);
    }
    Ok(())
}

pub(super) fn remote_shell_quote(arg: &OsStr) -> Result<String> {
    let value = arg
        .to_str()
        .context("ssh remote command arguments must be valid UTF-8")?;
    if value.is_empty() {
        return Ok("''".to_string());
    }
    Ok(format!("'{}'", value.replace('\'', "'\\''")))
}

pub(super) fn existing_run_context(run_dir: PathBuf) -> RunContext {
    let run_id = run_id_from_run_dir(&run_dir);
    RunContext { run_id, run_dir }
}

pub(super) fn write_json_artifact<T: Serialize>(
    run: &RunContext,
    path: &Path,
    value: &T,
) -> Result<String> {
    write_json_pretty(path, value)?;
    Ok(run.artifact_uri(path)?)
}

pub(super) fn evidence_store_for_run(run: &RunContext) -> Result<EvidenceStore> {
    Ok(EvidenceStore::open(std::slice::from_ref(&run.run_dir))?)
}

pub(super) fn write_text_file(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create artifact directory {}", parent.display()))?;
    }
    fs::write(path, value).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub(super) fn print_artifact<T: Serialize>(run: &RunContext, path: &Path, value: T) -> Result<()> {
    let artifact_ref = run.artifact_uri(path)?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value,
    })
}

pub(crate) fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub(super) fn path_ref(path: &Path) -> String {
    path.display().to_string()
}
