use super::super::*;
use super::common::*;
use adc_lab_core::ids::now_unix_ms;

pub(crate) fn command_report_operating_contract(args: OperatingContractCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let mut run_dirs = vec![run.run_dir.clone()];
    run_dirs.extend(args.include_runs.clone());
    let store = EvidenceStore::open(&run_dirs)?;
    let v2_contract =
        evaluate_operating_contract_v2(&store, run.run_id.clone(), args.target_id.clone());
    let v2_contract_path = run
        .run_dir
        .join("reports/target_operating_contract.v2.json");
    let v2_contract_ref = write_json_artifact(&run, &v2_contract_path, &v2_contract)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: args.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.target_operating_contract".to_string(),
            operation_id: None,
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&v2_contract.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;

    print_json(&serde_json::json!({
        "target_operating_contract_ref": v2_contract_ref,
        "included_run_count": args.include_runs.len(),
        "target_operating_contract": v2_contract
    }))
}

pub(crate) fn command_report_pack(args: ReportPackCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let (report, path, _) = persist_run_report(&run, args.target_id.clone())?;
    let now = now_unix_ms();
    persist_run_manifest(&run, args.target_id.clone(), args.target, now, now)?;
    print_artifact(&run, &path, report)
}

pub(crate) fn command_report_operating_point(args: ReportPackCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let (report, path, _) = persist_run_report(&run, args.target_id)?;
    print_artifact(&run, &path, report)
}
