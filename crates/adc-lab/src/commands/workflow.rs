use super::super::*;
use super::common::*;
use adc_lab_core::ids::new_id;

pub(crate) fn command_workflow_recommend(args: WorkflowRecommendCommand) -> Result<()> {
    validate_workflow_goal(&args.goal)?;
    let run = match args.run_dir.clone() {
        Some(path) => Some(create_or_open_run(Some(path))?),
        None => None,
    };
    let run_id = run
        .as_ref()
        .map(|run| run.run_id.clone())
        .unwrap_or_else(|| new_id("WORKFLOW-OFFLINE"));
    let recommendation =
        target_operating_contract_workflow_recommendation(WorkflowRecommendationInput {
            run_id,
            goal: args.goal,
            target: args.target,
            target_id: args.target_id,
            target_class: args.target_class,
            recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
        })?;
    let output_path = args.out.or_else(|| {
        run.as_ref()
            .map(|run| run.run_dir.join("workflows/recommendation.v2.json"))
    });

    let Some(path) = output_path else {
        return print_json(&recommendation);
    };

    write_json_pretty(&path, &recommendation)?;
    let artifact_ref = if let Some(run) = &run {
        if path.starts_with(&run.run_dir) {
            let artifact_ref = run.artifact_uri(&path)?;
            append_audit_event(
                run,
                AuditInput {
                    target_id: recommendation.target_id.clone(),
                    actor: Actor::codex(),
                    operation: "workflow.recommend".to_string(),
                    operation_id: Some(recommendation.id.clone()),
                    risk_tier: RiskTier::Tier0ReadOnlyObservation,
                    approval_ref: None,
                    restore_lease_ref: None,
                    result: "recommended".to_string(),
                },
            )?;
            artifact_ref
        } else {
            path_ref(&path)
        }
    } else {
        path_ref(&path)
    };

    print_json(&ArtifactOutput {
        artifact_ref,
        value: recommendation,
    })
}
