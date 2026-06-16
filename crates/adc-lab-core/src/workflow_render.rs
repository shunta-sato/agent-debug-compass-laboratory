use crate::evidence::Artifact;
use crate::workflow::{
    WorkflowCollectPlanPayload, WorkflowRecommendationPayload, COLLECT_PLAN_DEFERRED_NEXT_STEP,
};

pub fn render_collect_plan_agent_instructions(
    collect_plan: &Artifact<WorkflowCollectPlanPayload>,
) -> String {
    let payload = &collect_plan.payload;
    let mut out = String::new();
    out.push_str("# adc-lab Collect Plan Instructions\n\n");
    out.push_str("Use this workflow.collect_plan artifact as the executable handoff contract.\n\n");
    out.push_str(&format!("- goal: `{}`\n", payload.goal));
    out.push_str(&format!(
        "- effective_profile: `{}`\n",
        payload.effective_profile
    ));
    out.push_str(&format!(
        "- profile_depth: `{}`\n",
        payload.profile_depth.as_str()
    ));
    out.push_str(&format!("- profile_summary: {}\n", payload.profile_summary));
    out.push_str(&format!("- coverage: {}\n", payload.coverage));
    out.push_str(&format!("- safety_caps: {}\n", payload.safety_caps));
    out.push_str(&format!("- workflow_id: `{}`\n", payload.workflow_id));
    out.push_str(&format!("- target: `{}`\n", payload.target));
    out.push_str(&format!("- target_id: `{}`\n", payload.target_id));
    out.push_str(&format!("- target_class: `{}`\n", payload.target_class));
    out.push_str(&format!(
        "- planned_run_dir: `{}`\n\n",
        payload.planned_run_dir
    ));
    out.push_str("Do not fall back to a static prompt or hand-written shell harness when this collect plan is available.\n");
    out.push_str(&format!("{}\n", payload.claim_boundary));
    out.push_str("If a required workflow surface is missing, stop and report adc-lab version/capability mismatch.\n");
    out.push_str("Do not infer artifact relationships from path names, timestamps, or directory co-presence.\n\n");
    if payload.target.starts_with("ssh://") {
        out.push_str("## SSH Target Runner Boundary\n\n");
        out.push_str("Set `ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target` when the release installer default is not on the non-interactive SSH PATH.\n");
        out.push_str("The release installer installs user binaries under `~/.local/bin` by default; non-interactive SSH PATH may omit that directory.\n");
        out.push_str("Do not treat adc-lab or adc-lab-target as missing merely because command -v fails under the default non-interactive SSH PATH.\n\n");
    }
    if payload
        .steps
        .iter()
        .any(|step| step.execution_location == "target_local")
    {
        out.push_str("For target_local execution, run `export PATH=\"$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH\"` before executing target-local argv steps.\n\n");
    }
    out.push_str("## Steps\n\n");
    for step in &payload.steps {
        out.push_str(&format!("### `{}`\n\n", step.step_id));
        out.push_str(&format!("- phase: `{}`\n", step.phase));
        out.push_str(&format!(
            "- execution_location: `{}`\n",
            step.execution_location
        ));
        out.push_str(&format!("- claim_gate: `{}`\n", step.claim_gate));
        out.push_str(&format!(
            "- continue_on: `{}`\n",
            serde_json::to_string(&step.continue_on).unwrap_or_else(|_| "[]".to_string())
        ));
        out.push_str(&format!(
            "- stop_on: `{}`\n",
            serde_json::to_string(&step.stop_on).unwrap_or_else(|_| "[]".to_string())
        ));
        out.push_str(&format!(
            "- argv: `{}`\n",
            serde_json::to_string(&step.command_argv).unwrap_or_else(|_| "[]".to_string())
        ));
        if !step.expected_artifact_paths_or_globs.is_empty() {
            out.push_str("- expected_paths:\n");
            for path in &step.expected_artifact_paths_or_globs {
                out.push_str(&format!("  - `{path}`\n"));
            }
        }
        out.push_str(&format!("- summary: {}\n\n", step.human_summary));
    }
    out.push_str("Packaging steps are handoff steps, not target evidence. Packaging failure blocks final handoff completion but does not change measurement validity.\n");
    out
}

pub fn render_codex_agent_instructions(
    recommendation: &Artifact<WorkflowRecommendationPayload>,
    collect_plan_available: bool,
) -> String {
    let payload = &recommendation.payload;
    let next_step = if collect_plan_available {
        "run adc-lab collect plan, then follow the emitted argv-array steps"
    } else {
        COLLECT_PLAN_DEFERRED_NEXT_STEP
    };
    let mut out = String::new();
    out.push_str("# adc-lab Controller Agent Instructions\n\n");
    out.push_str("## Workflow Authority\n\n");
    out.push_str("Use adc-lab workflow outputs as the source of truth for Target Operating Contract full-set collection.\n\n");
    out.push_str(&format!("- goal: `{}`\n", payload.goal));
    out.push_str(&format!(
        "- effective_profile: `{}`\n",
        payload.effective_profile
    ));
    out.push_str(&format!(
        "- profile_depth: `{}`\n",
        payload.profile_depth.as_str()
    ));
    out.push_str(&format!("- profile_summary: {}\n", payload.profile_summary));
    out.push_str(&format!("- coverage: {}\n", payload.coverage));
    out.push_str(&format!("- safety_caps: {}\n", payload.safety_caps));
    out.push_str(&format!("- workflow_id: `{}`\n", payload.workflow_id));
    out.push_str(&format!(
        "- adc_lab_version: `{}`\n",
        payload.controller_adc_lab.version
    ));
    out.push_str(&format!(
        "- adc_lab_git_sha: `{}`\n",
        payload.controller_adc_lab.git_sha
    ));
    out.push_str(&format!("- target: `{}`\n", payload.target));
    out.push_str(&format!("- target_id: `{}`\n", payload.target_id));
    out.push_str(&format!("- target_class: `{}`\n\n", payload.target_class));

    out.push_str("## Source Of Truth Chain\n\n");
    for item in &payload.source_of_truth_chain {
        out.push_str(&format!("- `{item}`\n"));
    }
    out.push('\n');

    out.push_str("## Required Workflow\n\n");
    for item in &payload.must_use {
        out.push_str(&format!("- {item}\n"));
    }
    out.push_str(&format!("- next_step: {next_step}\n\n"));

    out.push_str("## Forbidden Fallbacks\n\n");
    out.push_str("- Do not fall back to a static prompt or hand-written shell harness when adc-lab workflow surfaces are available.\n");
    out.push_str("- If a required workflow surface is missing, stop and report adc-lab version/capability mismatch.\n");
    out.push_str("- Do not infer artifact relationships from filenames, timestamps, or directory co-presence.\n");
    out.push_str("- Do not use raw primitive control artifacts for controlled-governor full-set claims without matching report.run_validation.\n\n");

    if payload.target.starts_with("ssh://") {
        out.push_str("## SSH Target Runner Boundary\n\n");
        out.push_str("- Set `ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target` when the installed target runner is under the release installer default path.\n");
        out.push_str("- The release installer installs `adc-lab-target` under `~/.local/bin`; non-interactive SSH PATH may omit that directory.\n");
        out.push_str("- Do not treat adc-lab or adc-lab-target as missing merely because command -v fails under the default non-interactive SSH PATH.\n");
        out.push_str("- For target_local execution, run `export PATH=\"$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH\"` before executing target-local argv steps.\n\n");
    }

    out.push_str("## Expected Outputs\n\n");
    for item in &payload.expected_outputs {
        out.push_str(&format!("- `{item}`\n"));
    }
    out.push('\n');

    out.push_str("## Claim Boundaries\n\n");
    out.push_str("- This prompt is not target measurement evidence.\n");
    out.push_str(&format!("- {}\n", payload.claim_boundary));
    out.push_str("- Insufficient, refused, contaminated, not applicable, and unknown outcomes remain valid evidence boundaries.\n");
    out.push_str("- Version skew blocks full-set measured claims unless a later validation artifact explicitly records the allowed exploratory override, and even then full-set selection remains not ready.\n");
    out.push_str("- No Agent root shell, arbitrary sysfs writes, remote privileged apply/restore, Pi4/Pi5 selection claim, or production-style claim is authorized by this prompt.\n");
    out
}
