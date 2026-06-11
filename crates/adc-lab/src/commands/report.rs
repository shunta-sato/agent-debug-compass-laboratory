use super::super::*;

pub(crate) fn command_report_operating_contract(args: OperatingContractCommand) -> Result<()> {
    let run = existing_run_context(args.run);
    let inventory = platform_mechanism_inventory_for_run(
        &run.run_dir,
        args.target_id.clone(),
        args.target_class.clone(),
    )?;
    let plan = boundary_probe_plan(args.target_id.clone(), args.target_class.clone());
    let coupling = resource_coupling_report_for_run(&run.run_dir, args.target_id.clone())?;

    let inventory_path = run
        .run_dir
        .join("reports/platform_mechanism_inventory.json");
    let plan_path = run.run_dir.join("reports/boundary_probe_plan.json");
    let coupling_path = run.run_dir.join("reports/resource_coupling_report.json");
    let inventory_ref = write_json_artifact(&run, &inventory_path, &inventory)?;
    let plan_ref = write_json_artifact(&run, &plan_path, &plan)?;
    let coupling_ref = write_json_artifact(&run, &coupling_path, &coupling)?;

    let contract =
        target_operating_contract_for_run(&run.run_dir, args.target_id.clone(), args.target_class)?;
    let contract_path = run.run_dir.join("reports/target_operating_contract.json");
    let contract_ref = write_json_artifact(&run, &contract_path, &contract)?;
    let mut run_set_ref = None;
    let mut multi_run_contract_ref = None;
    let mut multi_run_contract = None;
    if !args.include_runs.is_empty() {
        let run_set = run_set_manifest_for_runs(
            &run.run_dir,
            &args.include_runs,
            args.target_id.clone(),
            contract.target_class.clone(),
        )?;
        let run_set_path = run.run_dir.join("reports/run_set_manifest.json");
        let written_run_set_ref = write_json_artifact(&run, &run_set_path, &run_set)?;
        let multi = multi_run_operating_contract_for_runs(
            &run.run_dir,
            &args.include_runs,
            args.target_id.clone(),
            contract.target_class.clone(),
            Some(written_run_set_ref.clone()),
        )?;
        let multi_path = run
            .run_dir
            .join("reports/multi_run_operating_contract.json");
        let written_multi_ref = write_json_artifact(&run, &multi_path, &multi)?;
        run_set_ref = Some(written_run_set_ref);
        multi_run_contract_ref = Some(written_multi_ref);
        multi_run_contract = Some(multi);
    }
    let inventory_status = if inventory
        .mechanisms
        .iter()
        .any(|mechanism| mechanism.evidence_status == ContractEvidenceStatus::Insufficient)
    {
        ContractEvidenceStatus::Insufficient
    } else {
        ContractEvidenceStatus::MeasuredPartial
    };

    for (operation, result) in [
        (
            "report.platform_mechanism_inventory",
            serde_json::to_string(&inventory_status).unwrap_or_else(|_| "insufficient".to_string()),
        ),
        ("report.boundary_probe_plan", "planned".to_string()),
        (
            "report.resource_coupling",
            serde_json::to_string(&coupling.report_status)
                .unwrap_or_else(|_| "unknown".to_string()),
        ),
        (
            "report.target_operating_contract",
            serde_json::to_string(&contract.contract_status)
                .unwrap_or_else(|_| "unknown".to_string()),
        ),
    ] {
        append_audit_event(
            &run,
            AuditInput {
                target_id: args.target_id.clone(),
                actor: Actor::codex(),
                operation: operation.to_string(),
                operation_id: None,
                risk_tier: RiskTier::Tier0ReadOnlyObservation,
                approval_ref: None,
                restore_lease_ref: None,
                result: result.trim_matches('"').to_string(),
            },
        )?;
    }
    if let Some(multi) = multi_run_contract.as_ref() {
        for (operation, result) in [
            ("report.run_set_manifest", "recorded".to_string()),
            (
                "report.multi_run_operating_contract",
                serde_json::to_string(&multi.contract_status)
                    .unwrap_or_else(|_| "unknown".to_string()),
            ),
        ] {
            append_audit_event(
                &run,
                AuditInput {
                    target_id: args.target_id.clone(),
                    actor: Actor::codex(),
                    operation: operation.to_string(),
                    operation_id: None,
                    risk_tier: RiskTier::Tier0ReadOnlyObservation,
                    approval_ref: None,
                    restore_lease_ref: None,
                    result: result.trim_matches('"').to_string(),
                },
            )?;
        }
    }

    let store = evidence_store_for_run(&run)?;
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
            operation: "report.target_operating_contract.v2".to_string(),
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
        "platform_mechanism_inventory_ref": inventory_ref,
        "boundary_probe_plan_ref": plan_ref,
        "resource_coupling_report_ref": coupling_ref,
        "target_operating_contract_ref": v2_contract_ref,
        "v1_target_operating_contract_ref": contract_ref,
        "run_set_manifest_ref": run_set_ref,
        "multi_run_operating_contract_ref": multi_run_contract_ref,
        "multi_run_operating_contract": multi_run_contract,
        "target_operating_contract": v2_contract
    }))
}
