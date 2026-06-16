use crate::run_validation::GovernorValidity;
use crate::workflow::WorkflowCollectPlanStep;

const THERMAL_ABORT_C: &str = "75";

pub(crate) fn cpu_thermal_characterization_steps(
    target: &str,
    run_dir: &str,
) -> Vec<WorkflowCollectPlanStep> {
    vec![
        observe_step(
            "observe_baseline_60s",
            target,
            run_dir,
            "60s",
            "baseline_context",
            "Collect a 60s passive baseline before CPU/thermal characterization.",
        ),
        observe_step(
            "observe_baseline_300s",
            target,
            run_dir,
            "300s",
            "baseline_context",
            "Collect a 300s passive baseline; this is still context, not sustained safety evidence.",
        ),
        load_step(
            "cpu_ladder_1_worker_60s",
            target,
            run_dir,
            "1",
            "60s",
            "ladder_set_cooldown_after_4_worker_point",
            "Run the 1-worker point in the bounded CPU/thermal ladder.",
        ),
        load_step(
            "cpu_ladder_2_worker_60s",
            target,
            run_dir,
            "2",
            "60s",
            "ladder_set_cooldown_after_4_worker_point",
            "Run the 2-worker point in the bounded CPU/thermal ladder.",
        ),
        load_step(
            "cpu_ladder_4_worker_60s",
            target,
            run_dir,
            "4",
            "60s",
            "cooldown_after_ladder",
            "Run the 4-worker point in the bounded CPU/thermal ladder.",
        ),
        observe_step(
            "cooldown_after_ladder",
            target,
            run_dir,
            "60s",
            "cooldown_after_ladder",
            "Observe cooldown after the CPU ladder before repeatability trials.",
        ),
        load_step(
            "cpu_repeatability_4_worker_1",
            target,
            run_dir,
            "4",
            "60s",
            "cooldown_after_repeatability_1",
            "Run 4-worker repeatability trial 1 of 3.",
        ),
        observe_step(
            "cooldown_after_repeatability_1",
            target,
            run_dir,
            "60s",
            "cooldown_between_repeatability_trials",
            "Observe cooldown after repeatability trial 1.",
        ),
        load_step(
            "cpu_repeatability_4_worker_2",
            target,
            run_dir,
            "4",
            "60s",
            "cooldown_after_repeatability_2",
            "Run 4-worker repeatability trial 2 of 3.",
        ),
        observe_step(
            "cooldown_after_repeatability_2",
            target,
            run_dir,
            "60s",
            "cooldown_between_repeatability_trials",
            "Observe cooldown after repeatability trial 2.",
        ),
        load_step(
            "cpu_repeatability_4_worker_3",
            target,
            run_dir,
            "4",
            "60s",
            "cooldown_before_sustained_bounded_load",
            "Run 4-worker repeatability trial 3 of 3.",
        ),
        observe_step(
            "cooldown_before_sustained_bounded_load",
            target,
            run_dir,
            "60s",
            "cooldown_before_sustained_bounded_load",
            "Observe cooldown before the sustained bounded load step.",
        ),
        load_step(
            "sustained_bounded_load_300s",
            target,
            run_dir,
            "4",
            "300s",
            "cooldown_after_sustained_load",
            "Run a 4-worker 300s sustained bounded load; this does not support 24h sustained safety.",
        ),
        observe_step(
            "cooldown_after_sustained_load",
            target,
            run_dir,
            "120s",
            "cooldown_after_sustained_load",
            "Observe cooldown after the 300s sustained bounded load.",
        ),
    ]
}

fn observe_step(
    step_id: &str,
    target: &str,
    run_dir: &str,
    duration: &str,
    cooldown_expectation: &str,
    human_summary: &str,
) -> WorkflowCollectPlanStep {
    step(
        step_id,
        "read_only",
        vec![
            "adc-lab",
            "observe",
            "--target",
            target,
            "--duration",
            duration,
            "--sample-interval",
            "1s",
            "--run-dir",
            run_dir,
            "--json",
        ],
        vec!["observation"],
        "cpu_thermal_context_not_claim_producing",
        vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
        vec![GovernorValidity::Refused, GovernorValidity::Unknown],
        vec![
            format!(
                "duration={duration}; workers=not_applicable; abort_temp_c=not_applicable_read_only; cooldown_expectation={cooldown_expectation}"
            ),
            "passive observation is context, not a controlled CPU/thermal claim by itself"
                .to_string(),
        ],
        human_summary,
    )
}

fn load_step(
    step_id: &str,
    target: &str,
    run_dir: &str,
    workers: &str,
    duration: &str,
    cooldown_expectation: &str,
    human_summary: &str,
) -> WorkflowCollectPlanStep {
    let mut notes = vec![
        format!(
            "duration={duration}; workers={workers}; abort_temp_c={THERMAL_ABORT_C}C; cooldown_expectation={cooldown_expectation}"
        ),
        "thermal abort threshold and operator abort handling bound this experiment".to_string(),
    ];
    let claim_gate = if duration == "300s" {
        notes.push(
            "300s bounded evidence does not support 24h sustained thermal safety".to_string(),
        );
        notes.push("optional approved 900s profile remains disabled by default".to_string());
        "sustained_300s_not_24h_safety"
    } else {
        "cpu_ladder_or_repeatability_not_production_safety"
    };

    step(
        step_id,
        "load",
        vec![
            "adc-lab",
            "load",
            "cpu",
            "--target",
            target,
            "--workers",
            workers,
            "--duration",
            duration,
            "--abort-temp-c",
            THERMAL_ABORT_C,
            "--run-dir",
            run_dir,
            "--json",
        ],
        vec!["load"],
        claim_gate,
        vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
        vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
        notes,
        human_summary,
    )
}

#[allow(clippy::too_many_arguments)]
fn step<S>(
    step_id: &str,
    phase: &str,
    command_argv: Vec<S>,
    expected_artifact_kinds: Vec<&str>,
    claim_gate: &str,
    continue_on: Vec<GovernorValidity>,
    stop_on: Vec<GovernorValidity>,
    validation_after_step: Vec<String>,
    human_summary: &str,
) -> WorkflowCollectPlanStep
where
    S: Into<String>,
{
    WorkflowCollectPlanStep {
        step_id: step_id.to_string(),
        phase: phase.to_string(),
        command_argv: command_argv.into_iter().map(Into::into).collect(),
        working_directory_policy: "repository_root".to_string(),
        execution_location: "controller".to_string(),
        requires_target_local: false,
        requires_controller: true,
        requires_approval_policy: false,
        requires_privileged_helper: false,
        expected_artifact_kinds: expected_artifact_kinds
            .into_iter()
            .map(str::to_string)
            .collect(),
        expected_artifact_paths_or_globs: Vec::new(),
        claim_gate: claim_gate.to_string(),
        continue_on,
        stop_on,
        validation_after_step,
        human_summary: human_summary.to_string(),
    }
}
