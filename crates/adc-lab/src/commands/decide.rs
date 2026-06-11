use super::super::*;

pub(crate) fn command_decide_suitability(args: SuitabilityCommand) -> Result<()> {
    let target_contract = read_target_contract(&args.target_contract)?;
    let workload: WorkloadDemandProfile = read_json(&args.workload_demand)?;
    let policy: SuitabilityPolicy = read_yaml(&args.policy)?;
    let decision = decide_suitability(
        &args.target_run,
        &target_contract,
        &workload,
        &policy,
        path_ref(&args.target_contract),
        path_ref(&args.workload_demand),
        path_ref(&args.policy),
    )?;
    let artifact = suitability_artifact_from_legacy_decision_v2(
        &decision,
        run_id_from_run_dir(&args.target_run),
    );
    write_json_pretty(&args.out, &artifact)?;
    append_audit_event(
        &existing_run_context(args.target_run.clone()),
        AuditInput {
            target_id: decision.target_id.clone(),
            actor: Actor::codex(),
            operation: "decide.suitability".to_string(),
            operation_id: Some(decision.decision_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&artifact.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    if args.json {
        print_json(&artifact)?;
    } else {
        println!("{}", args.out.display());
    }
    Ok(())
}

fn read_target_contract(path: &std::path::Path) -> Result<TargetOperatingContract> {
    let value: serde_json::Value = read_json(path)?;
    if value.get("schema").and_then(|schema| schema.as_str()) == Some("lab.artifact.v2") {
        let artifact: Artifact<OperatingContractPayload> = serde_json::from_value(value)?;
        Ok(legacy_contract_from_v2_artifact(
            &artifact,
            "projected-v2-contract",
        ))
    } else {
        Ok(serde_json::from_value(value)?)
    }
}
