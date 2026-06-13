use super::super::*;
use super::common::*;
use adc_lab_core::ids::new_id;
use clap::{Args, Subcommand, ValueEnum};
use serde::Serialize;

#[derive(Debug, Subcommand)]
pub(crate) enum AgentCommand {
    Instructions(AgentInstructionsCommand),
}

#[derive(Debug, Args)]
pub(crate) struct AgentInstructionsCommand {
    #[arg(long, default_value = "target-operating-contract-fullset")]
    goal: String,
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long, default_value = "local-target")]
    target_id: String,
    #[arg(long, default_value = "unknown-target-class")]
    target_class: String,
    #[arg(long, value_enum, default_value_t = AgentInstructionsFormatArg::Codex)]
    format: AgentInstructionsFormatArg,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum AgentInstructionsFormatArg {
    Codex,
}

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

pub(crate) fn command_agent(command: AgentCommand) -> Result<()> {
    match command {
        AgentCommand::Instructions(args) => command_agent_instructions(args),
    }
}

fn command_agent_instructions(args: AgentInstructionsCommand) -> Result<()> {
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
    let instructions = render_codex_agent_instructions(&recommendation, true);
    write_text_file(&args.out, &instructions)?;
    print_json(&AgentInstructionsOutput {
        instructions_path: path_ref(&args.out),
        goal: recommendation.payload.goal,
        workflow_id: recommendation.payload.workflow_id,
        adc_lab_version: recommendation.payload.controller_adc_lab.version,
        expected_outputs: recommendation.payload.expected_outputs,
        next_step: "run adc-lab collect plan, then follow the emitted argv-array steps".to_string(),
    })
}
