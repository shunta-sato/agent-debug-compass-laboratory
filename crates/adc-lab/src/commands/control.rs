use super::super::*;
use super::common::*;
use adc_lab_core::ids::{new_id, now_unix_ms};
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

pub(crate) fn command_control_governor_sweep_prepare(
    args: GovernorSweepPrepareCommand,
) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let governors = normalize_sweep_governors(args.governors.clone())?;
    let mut artifact = governor_sweep_policy_artifact(
        &run,
        GovernorSweepPolicyPayload {
            target_id: target.target_id.clone(),
            governors,
            duration_seconds_max: args.duration_seconds_max,
            thermal_celsius_abort: args.thermal_celsius_abort,
            expires_at_unix_ms: now_unix_ms() + args.expires_in_seconds.saturating_mul(1000),
            requested_by: Actor {
                kind: ActorKind::Agent,
                id: args.requested_by,
            },
            approved_by: None,
            policy_state: GovernorSweepPolicyState::Requested,
            policy_digest: String::new(),
        },
    )?;
    artifact.status = Status::Insufficient;
    let path = args.out.unwrap_or_else(|| {
        run.run_dir
            .join("approvals/governor_sweep_policy_request.v2.json")
    });
    let artifact_ref = write_json_artifact(&run, &path, &artifact)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: target.target_id,
            actor: Actor::codex(),
            operation: "control.governor_sweep.prepare".to_string(),
            operation_id: Some(artifact.id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: Some(artifact_ref.clone()),
            restore_lease_ref: None,
            result: "requested".to_string(),
        },
    )?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value: artifact,
    })
}

pub(crate) fn command_control_governor_sweep_approve(
    args: GovernorSweepApproveCommand,
) -> Result<()> {
    let request: Artifact<GovernorSweepPolicyPayload> = read_json(&args.request)?;
    validate_policy_digest(&request)?;
    if request.kind != Kind::ControlGovernorSweepPolicy
        || request.payload.policy_state != GovernorSweepPolicyState::Requested
    {
        anyhow::bail!("governor sweep approval requires a requested sweep policy artifact");
    }
    let run = create_or_open_run(args.run_dir.or_else(|| infer_policy_run_dir(&args.request)))?;
    let mut payload = request.payload.clone();
    payload.policy_state = GovernorSweepPolicyState::Approved;
    payload.approved_by = Some(Actor {
        kind: ActorKind::Human,
        id: args.approved_by,
    });
    payload.policy_digest = governor_sweep_policy_digest(&payload)?;
    let artifact = Artifact::new(
        Kind::ControlGovernorSweepPolicy,
        new_id("GOVERNOR-SWEEP-POLICY"),
        run.run_id.clone(),
        payload.target_id.clone(),
        Status::Measured,
        payload,
        now_unix_ms(),
    );
    let path = args
        .out
        .unwrap_or_else(|| run.run_dir.join("approvals/governor_sweep_policy.v2.json"));
    let artifact_ref = write_json_artifact(&run, &path, &artifact)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: artifact.target_id.clone(),
            actor: Actor::codex(),
            operation: "control.governor_sweep.approve".to_string(),
            operation_id: Some(artifact.id.clone()),
            risk_tier: RiskTier::Tier2PrivilegedReversible,
            approval_ref: Some(artifact_ref.clone()),
            restore_lease_ref: None,
            result: "approved".to_string(),
        },
    )?;
    print_json(&ArtifactOutput {
        artifact_ref,
        value: artifact,
    })
}

pub(crate) fn command_control_governor_sweep_run(args: GovernorSweepRunCommand) -> Result<()> {
    let run = create_or_open_run(args.run_dir.clone())?;
    if !args.dry_run && args.approval_policy.is_none() {
        append_audit_event(
            &run,
            AuditInput {
                target_id: TargetSpec::parse(&args.target)?.target_id,
                actor: Actor::codex(),
                operation: "control.governor_sweep".to_string(),
                operation_id: None,
                risk_tier: RiskTier::Tier2PrivilegedReversible,
                approval_ref: None,
                restore_lease_ref: None,
                result: "refused".to_string(),
            },
        )?;
        anyhow::bail!(
            "real governor sweep requires an approved sweep policy; --approved-by alone is not authorization"
        );
    }
    let target = TargetSpec::parse(&args.target)?;
    let governors = normalize_sweep_governors(args.governors.clone())?;
    let policy = args
        .approval_policy
        .as_ref()
        .map(read_json::<Artifact<GovernorSweepPolicyPayload>>)
        .transpose()?;
    let policy_ref = args
        .approval_policy
        .as_ref()
        .map(|path| run.artifact_uri(path).unwrap_or_else(|_| path_ref(path)));
    if let Some(policy) = &policy {
        validate_approved_sweep_policy(
            policy,
            &target,
            &governors,
            args.duration_seconds_max,
            args.thermal_celsius_abort,
        )?;
        if let Some(approved_by) = args.approved_by.as_deref() {
            let policy_actor = policy
                .payload
                .approved_by
                .as_ref()
                .map(|actor| actor.id.as_str());
            if policy_actor != Some(approved_by) {
                anyhow::bail!("--approved-by does not match approved sweep policy");
            }
        }
    } else {
        anyhow::bail!("governor sweep run requires an approved sweep policy");
    }
    if !args.dry_run && !args.restore_after_each {
        anyhow::bail!("real governor sweep requires --restore-after-each");
    }
    for governor in &governors {
        let plan = new_cpufreq_plan(
            &run,
            &target,
            governor.clone(),
            args.duration_seconds_max,
            args.thermal_celsius_abort,
        );
        if let Err(refusal) = validate_control_plan(&plan) {
            anyhow::bail!("invalid generated sweep plan: {}", refusal.message);
        }
        let plan_path = run
            .run_dir
            .join("plans")
            .join(format!("{}.json", plan.plan_id));
        write_json_artifact(&run, &plan_path, &plan)?;
        let approved_by = policy
            .as_ref()
            .and_then(|policy| policy.payload.approved_by.as_ref())
            .map(|actor| actor.id.clone())
            .unwrap_or_else(|| "operator".to_string());
        let approval = new_approval_record(
            &plan,
            approved_by,
            format!("Approve governor sweep step for {governor}"),
        )?;
        let (approval_path, approval_ref) = write_approval_record(&run, &approval)?;
        append_audit_event(
            &run,
            AuditInput {
                target_id: plan.target_id.clone(),
                actor: Actor::codex(),
                operation: "control.governor_sweep.step".to_string(),
                operation_id: Some(plan.operation.operation_id.clone()),
                risk_tier: plan.risk_tier.clone(),
                approval_ref: Some(approval_ref.clone()),
                restore_lease_ref: None,
                result: "planned".to_string(),
            },
        )?;
        let dry_run_result = apply_control_plan(
            &plan,
            Some(&approval),
            &LinuxCpufreqBackend::default(),
            true,
        );
        let result = if args.dry_run || dry_run_result.status == ControlResultStatus::Refused {
            dry_run_result
        } else {
            invoke_helper_apply(&plan_path, Some(&approval_path))?
        };
        let control_ref = persist_control_result(&run, &result, Some(approval_ref))?;
        if result.status != ControlResultStatus::Applied {
            continue;
        }
        let load_result = persist_governor_sweep_load(&run, &target, &args, governor, control_ref);
        if args.restore_after_each {
            if let Some(lease) = &result.restore_lease {
                let lease_path = run
                    .run_dir
                    .join("leases")
                    .join(format!("{}.json", lease.lease_id));
                let restore = invoke_helper_restore(&lease_path)?;
                persist_control_result(&run, &restore, None)?;
                if restore.status == ControlResultStatus::Restored
                    && restore.target_id == LOCAL_TARGET_ID
                {
                    persist_restore_health_check(&run)?;
                }
            }
        }
        load_result?;
    }
    append_audit_event(
        &run,
        AuditInput {
            target_id: target.target_id,
            actor: Actor::codex(),
            operation: "control.governor_sweep".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier2PrivilegedReversible,
            approval_ref: policy_ref,
            restore_lease_ref: None,
            result: if args.dry_run { "dry_run" } else { "recorded" }.to_string(),
        },
    )?;
    let (validation, validation_ref, gaps_path) = persist_sweep_validation(&run, governors)?;
    print_json(&ArtifactOutput {
        artifact_ref: validation_ref,
        value: validation.clone(),
    })?;
    if !args.allow_non_measured && validation.payload.overall_validity != GovernorValidity::Measured
    {
        anyhow::bail!(
            "governor sweep produced non-measured evidence; see {}",
            gaps_path.display()
        );
    }
    Ok(())
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
) -> Result<String> {
    let result_path = run
        .run_dir
        .join("plans")
        .join(format!("{}.result.json", result.result_id));
    write_json_pretty(&result_path, result)?;
    let result_ref = run.artifact_uri(&result_path)?;
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
    Ok(result_ref)
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

fn normalize_sweep_governors(governors: Vec<String>) -> Result<Vec<String>> {
    let mut governors = governors
        .into_iter()
        .flat_map(|item| {
            item.split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    governors.sort();
    governors.dedup();
    if governors.is_empty() {
        anyhow::bail!("governor sweep requires at least one governor");
    }
    Ok(governors)
}

fn governor_sweep_policy_artifact(
    run: &RunContext,
    mut payload: GovernorSweepPolicyPayload,
) -> Result<Artifact<GovernorSweepPolicyPayload>> {
    let target_id = payload.target_id.clone();
    payload.policy_digest.clear();
    payload.policy_digest = governor_sweep_policy_digest(&payload)?;
    Ok(Artifact::new(
        Kind::ControlGovernorSweepPolicy,
        new_id("GOVERNOR-SWEEP-POLICY"),
        run.run_id.clone(),
        target_id,
        Status::Measured,
        payload,
        now_unix_ms(),
    ))
}

fn validate_policy_digest(policy: &Artifact<GovernorSweepPolicyPayload>) -> Result<()> {
    let expected = governor_sweep_policy_digest(&policy.payload)?;
    if policy.payload.policy_digest != expected {
        anyhow::bail!("governor sweep policy digest mismatch");
    }
    Ok(())
}

fn validate_approved_sweep_policy(
    policy: &Artifact<GovernorSweepPolicyPayload>,
    target: &TargetSpec,
    governors: &[String],
    duration_seconds_max: u64,
    thermal_celsius_abort: Option<f64>,
) -> Result<()> {
    validate_policy_digest(policy)?;
    if policy.kind != Kind::ControlGovernorSweepPolicy
        || policy.payload.policy_state != GovernorSweepPolicyState::Approved
        || policy.payload.approved_by.is_none()
    {
        anyhow::bail!("governor sweep run requires an approved sweep policy");
    }
    if policy.target_id != target.target_id || policy.payload.target_id != target.target_id {
        anyhow::bail!("approved sweep policy target does not match requested target");
    }
    for governor in governors {
        if !policy
            .payload
            .governors
            .iter()
            .any(|allowed| allowed == governor)
        {
            anyhow::bail!("approved sweep policy does not include governor {governor}");
        }
    }
    if policy.payload.duration_seconds_max < duration_seconds_max {
        anyhow::bail!("approved sweep policy duration bound is narrower than requested");
    }
    match (policy.payload.thermal_celsius_abort, thermal_celsius_abort) {
        (Some(approved), Some(requested)) if requested <= approved => {}
        (Some(_), None) | (None, None) => {}
        _ => anyhow::bail!("approved sweep policy thermal bound does not cover request"),
    }
    if policy.payload.expires_at_unix_ms < now_unix_ms() {
        anyhow::bail!("approved sweep policy is expired");
    }
    Ok(())
}

fn infer_policy_run_dir(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some("approvals") {
        return parent.parent().map(Path::to_path_buf);
    }
    None
}

fn persist_governor_sweep_load(
    run: &RunContext,
    target: &TargetSpec,
    args: &GovernorSweepRunCommand,
    governor: &str,
    control_ref: String,
) -> Result<String> {
    let duration = parse_duration(&args.load_duration)?;
    let plan = new_cpu_load_plan_with_operator_abort(
        target.target_id.clone(),
        args.load_workers,
        duration,
        args.load_abort_temp_c,
        false,
    )?;
    let plan_path = run
        .run_dir
        .join("loads")
        .join(format!("{}.plan.json", plan.load_id));
    write_json_artifact(run, &plan_path, &plan)?;
    let result = match target.transport {
        TargetTransport::Local => run_cpu_load_with_options(
            &plan,
            &CpuLoadRuntimeOptions {
                operator_abort_file: None,
            },
        )?,
        TargetTransport::Ssh => {
            anyhow::bail!("governor sweep load only supports local target in this phase")
        }
    };
    let result_status = result.status.clone();
    let target_id = result.target_id.clone();
    let relative = PathBuf::from("load").join(format!(
        "cpu.{}.v2.json",
        safe_artifact_id(&result.result_id, "LOAD-RESULT")
    ));
    let mut artifact = load_artifact_v2(run.run_id.clone(), result);
    attach_load_control_context(&mut artifact, Some(control_ref), Some(governor.to_string()));
    let mut store = evidence_store_for_run(run)?;
    let artifact_ref = store.write(&run.run_dir, &relative, &artifact)?;
    append_audit_event(
        run,
        AuditInput {
            target_id,
            actor: Actor::codex(),
            operation: "load.cpu".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: result_status,
        },
    )?;
    Ok(artifact_ref)
}

fn persist_sweep_validation(
    run: &RunContext,
    governors: Vec<String>,
) -> Result<(Artifact<RunValidationPayload>, String, PathBuf)> {
    let validation = validate_fullset_run(run, governors)?;
    let validation_path = run.run_dir.join("reports/run_validation.v2.json");
    let validation_ref = write_json_artifact(run, &validation_path, &validation)?;
    let gaps_path = run.run_dir.join("reports/GAPS.md");
    write_text_file(&gaps_path, &render_run_validation_gaps(&validation))?;
    append_audit_event(
        run,
        AuditInput {
            target_id: validation.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.validate_run".to_string(),
            operation_id: Some(validation.id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&validation.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    Ok((validation, validation_ref, gaps_path))
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
