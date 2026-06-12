use super::super::*;
use super::common::*;
use std::path::Path;

pub(crate) fn command_constraints_generate(args: ConstraintsGenerateCommand) -> Result<()> {
    let suitability: Artifact<SuitabilityPayload> = read_json(&args.decision)?;
    if suitability.schema != ARTIFACT_SCHEMA_V2 || suitability.kind != Kind::ReportSuitability {
        anyhow::bail!("constraints generate --decision must be a v2 report.suitability artifact");
    }
    let constraints = generate_constraints_artifact_v2(&suitability);
    let markdown = render_agent_constraints_markdown(&constraints, &path_ref(&args.decision));
    write_json_pretty(&args.out, &constraints)?;
    write_text_file(&args.agent_instructions_out, &markdown)?;
    if let Some(run) = run_context_for_report_artifact(&args.decision, &suitability.run_id) {
        append_audit_event(
            &run,
            AuditInput {
                target_id: suitability.target_id.clone(),
                actor: Actor::codex(),
                operation: "constraints.generate".to_string(),
                operation_id: Some(constraints.id.clone()),
                risk_tier: RiskTier::Tier0ReadOnlyObservation,
                approval_ref: None,
                restore_lease_ref: None,
                result: "generated".to_string(),
            },
        )?;
    }
    if args.json {
        print_json(&constraints)?;
    } else {
        println!("{}", args.out.display());
        println!("{}", args.agent_instructions_out.display());
    }
    Ok(())
}

fn run_context_for_report_artifact(path: &Path, run_id: &str) -> Option<RunContext> {
    let reports_dir = path.parent()?;
    if reports_dir.file_name().and_then(|name| name.to_str()) != Some("reports") {
        return None;
    }
    Some(RunContext {
        run_id: run_id.to_string(),
        run_dir: reports_dir.parent()?.to_path_buf(),
    })
}

pub(crate) fn command_constraints_check(args: ConstraintsCheckCommand) -> Result<()> {
    let constraints: Artifact<ConstraintsPayload> = read_json(&args.constraints)?;
    if constraints.schema != ARTIFACT_SCHEMA_V2 || constraints.kind != Kind::ReportConstraints {
        anyhow::bail!("constraints check --constraints must be a v2 report.constraints artifact");
    }
    let mode = match args.mode {
        ConstraintsCheckModeArg::CandidateContent => ConstraintCheckMode::CandidateContent,
        ConstraintsCheckModeArg::GeneratedConstraints => ConstraintCheckMode::GeneratedConstraints,
    };
    let result = check_constraints_v2(&constraints, &args.path, mode)?;
    print_json(&result)?;
    if result.payload.status == "fail" {
        anyhow::bail!("constraint check failed");
    }
    Ok(())
}
