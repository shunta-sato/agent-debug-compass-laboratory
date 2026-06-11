use super::super::*;
use super::common::*;
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) fn command_control_plan(args: ControlPlanCommand) -> Result<()> {
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

pub(crate) fn command_control_approve(args: ControlApproveCommand) -> Result<()> {
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

pub(crate) fn command_control_apply(args: ControlApplyCommand) -> Result<()> {
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

pub(crate) fn command_restore(args: RestoreCommand) -> Result<()> {
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
