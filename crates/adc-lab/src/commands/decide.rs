use super::super::*;

pub(crate) fn command_decide_suitability(args: SuitabilityCommand) -> Result<()> {
    let target_contract = read_operating_contract_artifact(&args.target_contract)?;
    let workload: WorkloadDemandProfile = read_json(&args.workload_demand)?;
    let policy: SuitabilityPolicy = read_yaml(&args.policy)?;
    let artifact = decide_suitability_artifact_v2(
        &args.target_run,
        &target_contract,
        &workload,
        &policy,
        SuitabilityArtifactContext {
            target_contract_ref: path_ref(&args.target_contract),
            workload_ref: path_ref(&args.workload_demand),
            policy_ref: path_ref(&args.policy),
            run_id: run_id_from_run_dir(&args.target_run),
        },
    )?;
    write_json_pretty(&args.out, &artifact)?;
    append_audit_event(
        &existing_run_context(args.target_run.clone()),
        AuditInput {
            target_id: artifact.target_id.clone(),
            actor: Actor::codex(),
            operation: "decide.suitability".to_string(),
            operation_id: Some(artifact.id.clone()),
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

fn read_operating_contract_artifact(
    path: &std::path::Path,
) -> Result<Artifact<OperatingContractPayload>> {
    let artifact: Artifact<OperatingContractPayload> = read_json(path)?;
    if artifact.schema != ARTIFACT_SCHEMA_V2 || artifact.kind != Kind::ReportOperatingContract {
        return Err(anyhow::anyhow!(
            "--target-contract must be a lab.artifact.v2 report.operating_contract artifact"
        ));
    }
    Ok(artifact)
}
