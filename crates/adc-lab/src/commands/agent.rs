use super::super::*;
use super::common::*;
use adc_lab_core::ids::new_id;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct AgentInstructionsOutput {
    instructions_path: String,
    goal: String,
    workflow_id: String,
    adc_lab_version: String,
    expected_outputs: Vec<String>,
    next_step: String,
}

pub(crate) fn command_agent_instructions(args: AgentInstructionsCommand) -> Result<()> {
    validate_workflow_goal(&args.goal)?;
    match args.format {
        AgentInstructionsFormatArg::Codex => {}
    }

    let recommendation =
        target_operating_contract_workflow_recommendation(WorkflowRecommendationInput {
            run_id: new_id("WORKFLOW-OFFLINE"),
            goal: args.goal,
            target: args.target,
            target_id: args.target_id,
            target_class: args.target_class,
            recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
        })?;
    let instructions = render_codex_agent_instructions(&recommendation, false);
    write_text_file(&args.out, &instructions)?;
    print_json(&AgentInstructionsOutput {
        instructions_path: path_ref(&args.out),
        goal: recommendation.payload.goal,
        workflow_id: recommendation.payload.workflow_id,
        adc_lab_version: recommendation.payload.controller_adc_lab.version,
        expected_outputs: recommendation.payload.expected_outputs,
        next_step: COLLECT_PLAN_DEFERRED_NEXT_STEP.to_string(),
    })
}
