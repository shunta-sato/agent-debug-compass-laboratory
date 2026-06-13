use super::super::*;
use super::common::*;
use clap::{Args, Subcommand};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Subcommand)]
pub(crate) enum CollectCommand {
    Plan(CollectPlanCommand),
}

#[derive(Debug, Args)]
pub(crate) struct CollectPlanCommand {
    #[arg(long, default_value = "target-operating-contract-fullset")]
    goal: String,
    #[arg(long, default_value = "local")]
    target: String,
    #[arg(long, default_value = "local-target")]
    target_id: String,
    #[arg(long, default_value = "unknown-target-class")]
    target_class: String,
    #[arg(long = "expected-governors", value_delimiter = ',')]
    expected_governors: Vec<String>,
    #[arg(long)]
    run_dir: Option<PathBuf>,
    #[arg(long)]
    out: PathBuf,
    #[arg(long = "agent-instructions-out")]
    agent_instructions_out: PathBuf,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
struct CollectPlanOutput {
    collect_plan_path: String,
    agent_instructions_path: String,
    workflow_id: String,
    planned_run_dir: String,
    step_count: usize,
    expected_final_artifacts: Vec<String>,
}

pub(crate) fn command_collect(command: CollectCommand) -> Result<()> {
    match command {
        CollectCommand::Plan(args) => command_collect_plan(args),
    }
}

fn command_collect_plan(args: CollectPlanCommand) -> Result<()> {
    let planned_run_dir = args
        .run_dir
        .clone()
        .unwrap_or_else(|| infer_planned_run_dir(&args.out));
    let workflow_recommendation_path = planned_run_dir.join("workflows/recommendation.v2.json");
    let workload_demand_path = planned_run_dir.join("inputs/workload_demand.json");
    let suitability_policy_path = planned_run_dir.join("inputs/suitability_policy.yaml");
    let plan = target_operating_contract_collect_plan(WorkflowCollectPlanInput {
        run_id: run_id_from_run_dir(&planned_run_dir),
        goal: args.goal,
        target: args.target,
        target_id: args.target_id,
        target_class: args.target_class,
        planned_run_dir: path_ref(&planned_run_dir),
        collect_plan_path: path_ref(&args.out),
        agent_instructions_path: path_ref(&args.agent_instructions_out),
        workflow_recommendation_path: path_ref(&workflow_recommendation_path),
        workflow_recommendation_ref: None,
        workflow_recommendation_digest: None,
        workload_demand_path: path_ref(&workload_demand_path),
        suitability_policy_path: path_ref(&suitability_policy_path),
        expected_governors: args.expected_governors,
        recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
    })?;
    write_json_pretty(&args.out, &plan)?;
    let instructions = render_collect_plan_agent_instructions(&plan);
    write_text_file(&args.agent_instructions_out, &instructions)?;
    let summary = CollectPlanOutput {
        collect_plan_path: path_ref(&args.out),
        agent_instructions_path: path_ref(&args.agent_instructions_out),
        workflow_id: plan.payload.workflow_id.clone(),
        planned_run_dir: plan.payload.planned_run_dir.clone(),
        step_count: plan.payload.steps.len(),
        expected_final_artifacts: plan.payload.expected_final_artifacts.clone(),
    };
    if args.json {
        print_json(&ArtifactOutput {
            artifact_ref: path_ref(&args.out),
            value: plan,
        })
    } else {
        print_json(&summary)
    }
}

fn infer_planned_run_dir(out: &Path) -> PathBuf {
    let Some(parent) = out.parent() else {
        return PathBuf::from(".");
    };
    if parent.file_name().and_then(|name| name.to_str()) == Some("workflows") {
        parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| parent.to_path_buf())
    } else {
        parent.to_path_buf()
    }
}
