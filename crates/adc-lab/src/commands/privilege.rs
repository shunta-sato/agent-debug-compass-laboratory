use super::super::*;
use super::common::*;

pub(crate) fn command_privilege_provider_status(args: TargetCommand) -> Result<()> {
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

pub(crate) fn command_privilege_doctor(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let local_target = matches!(target.transport, TargetTransport::Local);
    let report = privilege_doctor(target.target_id.clone(), local_target);
    let path = run.run_dir.join("privilege/privilege_doctor.json");
    write_json_artifact(&run, &path, &report)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: report.target_id.clone(),
            actor: Actor::codex(),
            operation: "privilege.doctor".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&report.status)
                .unwrap_or_else(|_| "\"unknown\"".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    print_artifact(&run, &path, report)
}

pub(crate) fn command_privilege_install_plan(args: PrivilegeInstallPlanCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let plan = privilege_install_plan(target.target_id.clone(), args.helper_bin.as_deref());
    let path = run.run_dir.join("privilege/privilege_install_plan.json");
    write_json_artifact(&run, &path, &plan)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: plan.target_id.clone(),
            actor: Actor::codex(),
            operation: "privilege.install_plan".to_string(),
            operation_id: Some(plan.plan_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "instruction_only".to_string(),
        },
    )?;
    print_artifact(&run, &path, plan)
}

pub(crate) fn command_privilege_uninstall_plan(args: TargetCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let plan = privilege_uninstall_plan(target.target_id.clone());
    let path = run.run_dir.join("privilege/privilege_uninstall_plan.json");
    write_json_artifact(&run, &path, &plan)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: plan.target_id.clone(),
            actor: Actor::codex(),
            operation: "privilege.uninstall_plan".to_string(),
            operation_id: Some(plan.plan_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: "instruction_only".to_string(),
        },
    )?;
    print_artifact(&run, &path, plan)
}
