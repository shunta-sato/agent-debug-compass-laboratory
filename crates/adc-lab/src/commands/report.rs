use super::super::*;
use super::common::*;
use adc_lab_core::ids::{new_id, now_unix_ms};
use anyhow::{bail, Context};

struct OperatingContractGateResult {
    gate: OperatingContractValidationGate,
    strict_failure: Option<String>,
    summary: serde_json::Value,
}

pub(crate) fn command_report_validate_run(args: ValidateRunCommand) -> Result<()> {
    let profile_depth = parse_workflow_profile_depth(args.profile_depth.as_deref())?;
    warn_legacy_workflow_profile(&args.profile);
    let profile = resolve_workflow_profile(&args.profile, profile_depth)?;
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
    let validation = validate_profile_run_set(
        RunValidationInput {
            subject_run: run.clone(),
            include_runs,
            requested_governors: args.expected_governors.clone(),
            workflow_recommendation_ref,
            collect_plan_ref,
            collect_plan_digest,
            target_id: Some(args.target_id.clone()),
            target_class: Some(args.target_class.clone()),
            allow_version_skew: args.allow_version_skew,
        },
        &profile.requested_profile,
        &profile.effective_profile,
    )?;
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
    let run = existing_run_context(args.run.clone());
    let include_runs = args
        .include_runs
        .iter()
        .cloned()
        .map(existing_run_context)
        .collect::<Vec<_>>();
    let mut run_contexts = vec![run.clone()];
    run_contexts.extend(include_runs.clone());
    let run_dirs = run_contexts
        .iter()
        .map(|run| run.run_dir.clone())
        .collect::<Vec<_>>();
    let store = EvidenceStore::open(&run_dirs)?;
    let gate_result = operating_contract_validation_gate(&args, &run, &run_contexts, &store)?;
    let report_run_present = store.iter(Kind::ReportRun).next().is_some();
    let mut v2_contract =
        evaluate_operating_contract_v2(&store, run.run_id.clone(), args.target_id.clone());
    apply_operating_contract_validation_gate(
        &mut v2_contract,
        &gate_result.gate,
        report_run_present,
    );
    let v2_contract_path = run
        .run_dir
        .join("reports/target_operating_contract.v2.json");
    let v2_contract_ref = write_json_artifact(&run, &v2_contract_path, &v2_contract)?;
    let expected_identity = run_set_identity_for_runs(&run_contexts)?;
    let evidence_ref_resolution = evidence_ref_resolution_artifact(
        &run,
        &store,
        expected_identity.subject_run_set_id,
        vec![v2_contract_ref.clone()],
        operating_contract_evidence_refs(&v2_contract),
        args.target_id.clone(),
    );
    let evidence_ref_resolution_path = run.run_dir.join("reports/evidence_ref_resolution.v2.json");
    let evidence_ref_resolution_ref = write_json_artifact(
        &run,
        &evidence_ref_resolution_path,
        &evidence_ref_resolution,
    )?;
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
    append_audit_event(
        &run,
        AuditInput {
            target_id: args.target_id.clone(),
            actor: Actor::codex(),
            operation: "report.evidence_ref_resolution".to_string(),
            operation_id: Some(evidence_ref_resolution.id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&evidence_ref_resolution.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;

    print_json(&serde_json::json!({
        "target_operating_contract_ref": v2_contract_ref,
        "evidence_ref_resolution_ref": evidence_ref_resolution_ref,
        "included_run_count": args.include_runs.len(),
        "validation_gate": gate_result.summary,
        "target_operating_contract": v2_contract,
        "evidence_ref_resolution": evidence_ref_resolution
    }))?;
    if let Some(reason) = gate_result.strict_failure {
        bail!("strict full-set operating-contract validation failed: {reason}");
    }
    Ok(())
}

fn evidence_ref_resolution_artifact(
    run: &RunContext,
    store: &EvidenceStore,
    subject_run_set_id: String,
    checked_artifact_refs: Vec<String>,
    references: Vec<String>,
    target_id: String,
) -> Artifact<EvidenceRefResolutionPayload> {
    let payload = store.evidence_ref_resolution_payload(
        subject_run_set_id,
        checked_artifact_refs,
        references,
    );
    let invalid_refs_empty = payload.invalid_refs.is_empty();
    let mut artifact = Artifact::new(
        Kind::ReportEvidenceRefResolution,
        new_id("EVIDENCE-REF-RESOLUTION"),
        run.run_id.clone(),
        target_id,
        if invalid_refs_empty {
            Status::Measured
        } else {
            Status::Insufficient
        },
        payload,
        now_unix_ms(),
    );
    artifact.evidence_refs = artifact
        .payload
        .resolutions
        .iter()
        .filter(|resolution| resolution.classification == EvidenceRefResolutionKind::Resolvable)
        .map(|resolution| resolution.reference.clone())
        .collect();
    artifact.evidence_refs.sort();
    artifact.evidence_refs.dedup();
    artifact.data_quality = DataQuality {
        level: if invalid_refs_empty {
            DataQualityLevel::Complete
        } else {
            DataQualityLevel::Degraded
        },
        notes: vec!["evidence refs checked against the opened run-set resolver".to_string()],
    };
    artifact
}

fn operating_contract_evidence_refs(contract: &Artifact<OperatingContractPayload>) -> Vec<String> {
    let mut refs = contract.evidence_refs.clone();
    refs.extend(
        contract
            .claims
            .iter()
            .flat_map(|claim| claim.evidence_refs.clone()),
    );
    refs.extend(
        contract
            .payload
            .evaluations
            .iter()
            .flat_map(|evaluation| evaluation.evidence_refs.clone()),
    );
    refs.sort();
    refs.dedup();
    refs
}

fn operating_contract_validation_gate(
    args: &OperatingContractCommand,
    subject_run: &RunContext,
    run_contexts: &[RunContext],
    store: &EvidenceStore,
) -> Result<OperatingContractGateResult> {
    let Some(validation_path) = &args.validation else {
        return Ok(operating_contract_gate_result(
            None,
            false,
            "missing_validation",
            "no --validation artifact was provided",
            args.strict_fullset,
        ));
    };

    let validation_ref = artifact_ref_for_optional_path(subject_run, validation_path)?;
    let validation: Artifact<RunValidationPayload> = read_json(validation_path)
        .with_context(|| format!("failed to read validation {}", validation_path.display()))?;
    if validation.kind != Kind::ReportRunValidation {
        return Ok(operating_contract_gate_result(
            Some(validation_ref),
            false,
            "invalid_validation_kind",
            "validation artifact kind is not report.run_validation",
            args.strict_fullset,
        ));
    }

    let expected_identity = run_set_identity_for_runs(run_contexts)?;
    let mut reasons = Vec::new();
    if validation.payload.subject_run_set_id != expected_identity.subject_run_set_id {
        reasons.push("subject_run_set_id does not match current run set".to_string());
    }
    if validation.payload.included_run_refs != expected_identity.included_run_refs {
        reasons.push("included_run_refs do not match current run set".to_string());
    }
    if !supported_validation_profile(&validation.payload.profile)
        || !(measured_validation_profile(&validation.payload.validation_profile)
            || validation.payload.validation_profile == FULLSET_PROFILE)
    {
        reasons.push("validation profile is not a supported workflow profile".to_string());
    }
    if validation.payload.workflow_id != WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023 {
        reasons.push(format!(
            "workflow_id is {}, expected {}",
            validation.payload.workflow_id, WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023
        ));
    }
    if validation.payload.target_id != args.target_id {
        reasons.push(format!(
            "target_id is {}, expected {}",
            validation.payload.target_id, args.target_id
        ));
    }
    if validation.payload.target_class != args.target_class {
        reasons.push(format!(
            "target_class is {}, expected {}",
            validation.payload.target_class, args.target_class
        ));
    }
    if !is_measured_fullset_validation(&validation) {
        reasons.push("validation artifact is not measured full-set evidence".to_string());
    }
    let indexed_ref_matches = store
        .iter(Kind::ReportRunValidation)
        .any(|meta| meta.artifact_ref == validation_ref);
    if !indexed_ref_matches {
        reasons.push("validation artifact is not indexed in the current run set".to_string());
    }

    if reasons.is_empty() {
        Ok(operating_contract_gate_result(
            Some(validation_ref),
            true,
            "measured",
            "validation artifact matches the current run set and is measured",
            false,
        ))
    } else {
        let reason = reasons.join("; ");
        Ok(operating_contract_gate_result(
            Some(validation_ref),
            false,
            "blocked",
            &reason,
            args.strict_fullset,
        ))
    }
}

fn operating_contract_gate_result(
    validation_ref: Option<String>,
    measured: bool,
    state: &str,
    reason: &str,
    strict: bool,
) -> OperatingContractGateResult {
    let gate = OperatingContractValidationGate {
        validation_ref: validation_ref.clone(),
        measured,
        reason: reason.to_string(),
    };
    OperatingContractGateResult {
        gate,
        strict_failure: strict.then_some(reason.to_string()).filter(|_| !measured),
        summary: serde_json::json!({
            "state": state,
            "measured": measured,
            "validation_ref": validation_ref,
            "reason": reason,
        }),
    }
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
