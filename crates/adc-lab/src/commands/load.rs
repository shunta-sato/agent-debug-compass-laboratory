use super::super::*;
use super::common::*;
use std::path::PathBuf;

pub(crate) fn command_load_cpu(args: LoadCpuCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    persist_target_runner_version_if_absent(&run, &target)?;
    let duration = parse_duration(&args.duration)?;
    let result = match target.transport {
        TargetTransport::Local => {
            let plan = new_cpu_load_plan_with_operator_abort(
                target.target_id.clone(),
                args.workers,
                duration,
                args.abort_temp_c,
                args.operator_abort_file.is_some(),
            )?;
            let plan_path = run
                .run_dir
                .join("loads")
                .join(format!("{}.plan.json", plan.load_id));
            write_json_artifact(&run, &plan_path, &plan)?;
            run_cpu_load_with_options(
                &plan,
                &CpuLoadRuntimeOptions {
                    operator_abort_file: args.operator_abort_file.clone(),
                },
            )?
        }
        TargetTransport::Ssh => load_cpu_ssh(
            &target,
            args.workers,
            duration,
            args.abort_temp_c,
            args.operator_abort_file.as_deref(),
        )?,
    };
    let result_status = result.status.clone();
    let target_id = result.target_id.clone();
    let relative = PathBuf::from("load").join(format!(
        "cpu.{}.v2.json",
        safe_artifact_id(&result.result_id, "LOAD-RESULT")
    ));
    let artifact = load_artifact_v2(run.run_id.clone(), result);
    let mut store = evidence_store_for_run(&run)?;
    let artifact_ref = store.write(&run.run_dir, &relative, &artifact)?;
    append_audit_event(
        &run,
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
    print_json(&ArtifactOutput {
        artifact_ref,
        value: artifact,
    })
}
