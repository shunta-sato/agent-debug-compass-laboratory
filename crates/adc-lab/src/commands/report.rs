use super::super::*;
use super::common::*;
use adc_lab_core::ids::now_unix_ms;
use anyhow::bail;

pub(crate) fn command_report_validate_run(args: ValidateRunCommand) -> Result<()> {
    if args.profile != FULLSET_PROFILE {
        bail!(
            "unsupported validation profile {}; expected {}",
            args.profile,
            FULLSET_PROFILE
        );
    }
    let run = existing_run_context(args.run);
    let include_runs = args
        .include_runs
        .into_iter()
        .map(existing_run_context)
        .collect::<Vec<_>>();
    let workflow_recommendation_ref = args
        .workflow_recommendation
        .as_deref()
        .map(|path| artifact_ref_for_optional_path(&run, path))
        .transpose()?;
    let collect_plan_ref = args
        .collect_plan
        .as_deref()
        .map(|path| artifact_ref_for_optional_path(&run, path))
        .transpose()?;
    let collect_plan_digest = args
        .collect_plan
        .as_deref()
        .map(digest_file_sha256)
        .transpose()?;
    let validation = validate_fullset_run_set(RunValidationInput {
        subject_run: run.clone(),
        include_runs,
        requested_governors: args.expected_governors.clone(),
        workflow_recommendation_ref,
        collect_plan_ref,
        collect_plan_digest,
        target_id: Some(args.target_id.clone()),
        target_class: Some(args.target_class.clone()),
        allow_version_skew: args.allow_version_skew,
    })?;
    let validation_path = args
        .out
        .unwrap_or_else(|| run.run_dir.join("reports/run_validation.v2.json"));
    let validation_ref = write_json_artifact(&run, &validation_path, &validation)?;
    let gaps_path = args
        .gaps_out
        .unwrap_or_else(|| run.run_dir.join("reports/GAPS.md"));
    write_text_file(&gaps_path, &render_run_validation_gaps(&validation))?;
    let has_non_measured = validation.payload.overall_validity != GovernorValidity::Measured;
    append_audit_event(
        &run,
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
    let output = ArtifactOutput {
        artifact_ref: validation_ref,
        value: serde_json::json!({
            "validation": validation,
            "gaps_ref": run.artifact_uri(&gaps_path)?,
        }),
    };
    print_json(&output)?;
    if !args.allow_non_measured && has_non_measured {
        bail!(
            "run validation contains non-measured governor evidence; see {}",
            gaps_path.display()
        );
    }
    Ok(())
}

fn artifact_ref_for_optional_path(run: &RunContext, path: &std::path::Path) -> Result<String> {
    if path.starts_with(&run.run_dir) {
        Ok(run.artifact_uri(path)?)
    } else {
        Ok(path_ref(path))
    }
}

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
