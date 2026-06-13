use adc_lab_core::ids::{new_id, now_unix_ms};
use adc_lab_core::{
    attach_load_control_context, load_artifact_v2, new_approval_record, new_cpufreq_plan,
    validate_fullset_run, validate_fullset_run_set, BuildInfo, ControlPlan, ControlResult,
    ControlResultStatus, GovernorValidity, HealthCheck, LoadRestoreOnAbortStatus,
    LoadSafetyMonitorResult, RunContext, RunValidationInput, TargetSpec, VersionSkewPolicyResult,
};
use adc_lab_core::{write_json_pretty, LoadResult};
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn stale_v021_prompt_governor_mislabel_is_never_measured() {
    let temp = tempfile::tempdir().unwrap();
    let run = test_run(temp.path());
    write_governor_trial(&run, "ondemand", true, None);
    let performance_ref = write_governor_trial(&run, "performance", false, None);
    write_governor_trial(&run, "powersave", true, Some(performance_ref));

    let validation = validate_fullset_run(
        &run,
        vec![
            "ondemand".to_string(),
            "performance".to_string(),
            "powersave".to_string(),
        ],
    )
    .unwrap();
    let result_for = |governor: &str| {
        validation
            .payload
            .governor_results
            .iter()
            .find(|result| result.governor == governor)
            .unwrap()
    };

    assert_eq!(result_for("ondemand").validity, GovernorValidity::Measured);
    assert_eq!(
        result_for("performance").validity,
        GovernorValidity::MeasuredPartial
    );
    assert_eq!(
        result_for("powersave").validity,
        GovernorValidity::Contaminated
    );
}

#[test]
fn mixed_controller_target_versions_block_fullset_claims_even_with_override() {
    let temp = tempfile::tempdir().unwrap();
    let controller_run = test_run(&temp.path().join("controller"));
    let target_run = test_run(&temp.path().join("target"));
    write_build_info(
        controller_run.run_dir.join("tools/adc-lab.version.json"),
        "adc-lab",
        "0.2.1",
        "controller-sha",
    );
    write_build_info(
        target_run.run_dir.join("tools/adc-lab.version.json"),
        "adc-lab",
        "0.2.2",
        "target-sha",
    );
    write_build_info(
        target_run.run_dir.join("tools/adc-lab-target.version.json"),
        "adc-lab-target",
        "0.2.2",
        "target-sha",
    );
    write_governor_trial(&target_run, "performance", true, None);

    let validation = validate_fullset_run_set(RunValidationInput {
        subject_run: controller_run.clone(),
        include_runs: vec![target_run.clone()],
        requested_governors: vec!["performance".to_string()],
        workflow_recommendation_ref: Some(
            "artifact://lab/runs/LAB-RUN-001/reports/workflow_recommendation.v2.json".to_string(),
        ),
        collect_plan_ref: None,
        collect_plan_digest: None,
        target_id: Some("target55".to_string()),
        target_class: Some("raspberry_pi_4".to_string()),
        allow_version_skew: false,
    })
    .unwrap();
    assert!(validation.payload.version_set.skew_detected);
    assert_eq!(
        validation.payload.version_skew_policy,
        VersionSkewPolicyResult::BlockedByVersionSkew
    );
    assert_eq!(
        validation.payload.governor_results[0].validity,
        GovernorValidity::Insufficient
    );
    assert_eq!(
        validation.payload.overall_validity,
        GovernorValidity::Insufficient
    );
    assert!(validation
        .payload
        .gaps
        .iter()
        .any(|gap| gap.code == "blocked_by_version_skew"));

    let override_validation = validate_fullset_run_set(RunValidationInput {
        subject_run: controller_run,
        include_runs: vec![target_run],
        requested_governors: vec!["performance".to_string()],
        workflow_recommendation_ref: None,
        collect_plan_ref: None,
        collect_plan_digest: None,
        target_id: Some("target55".to_string()),
        target_class: Some("raspberry_pi_4".to_string()),
        allow_version_skew: true,
    })
    .unwrap();
    assert_eq!(
        override_validation.payload.version_skew_policy,
        VersionSkewPolicyResult::OverrideRecordedStillBlocked
    );
    assert!(override_validation.payload.version_skew_override);
    assert_ne!(
        override_validation.payload.overall_validity,
        GovernorValidity::Measured
    );
}

#[test]
fn run_set_identity_changes_for_foreign_validation_sources() {
    let temp = tempfile::tempdir().unwrap();
    let first = test_run(&temp.path().join("first"));
    let second = test_run(&temp.path().join("second"));
    write_governor_trial(&first, "performance", true, None);
    write_governor_trial(&second, "performance", true, None);

    let first_validation = validate_fullset_run(&first, vec!["performance".to_string()]).unwrap();
    let second_validation = validate_fullset_run(&second, vec!["performance".to_string()]).unwrap();

    assert_ne!(
        first_validation.payload.subject_run_set_id,
        second_validation.payload.subject_run_set_id
    );
    assert_ne!(
        first_validation.payload.included_run_refs,
        second_validation.payload.included_run_refs
    );
}

fn test_run(root: &Path) -> RunContext {
    let run_dir = root.join("LAB-RUN-001");
    fs::create_dir_all(&run_dir).unwrap();
    RunContext {
        run_id: "LAB-RUN-001".to_string(),
        run_dir,
    }
}

fn write_governor_trial(
    run: &RunContext,
    governor: &str,
    include_restore: bool,
    load_control_ref_override: Option<String>,
) -> String {
    let target = TargetSpec::parse("local").unwrap();
    let plan = new_cpufreq_plan(run, &target, governor.to_string(), 60, Some(75.0));
    write_json_pretty(
        run.run_dir
            .join("plans")
            .join(format!("{}.json", plan.plan_id)),
        &plan,
    )
    .unwrap();
    let approval =
        new_approval_record(&plan, "operator".to_string(), "approve".to_string()).unwrap();
    write_json_pretty(
        run.run_dir
            .join("approvals")
            .join(format!("{}.json", approval.approval_id)),
        &approval,
    )
    .unwrap();
    let applied = applied_result(&plan);
    let applied_path = run
        .run_dir
        .join("plans")
        .join(format!("{}.result.json", applied.result_id));
    write_json_pretty(&applied_path, &applied).unwrap();
    let applied_ref = run.artifact_uri(&applied_path).unwrap();
    if include_restore {
        let restored = restored_result(&plan);
        write_json_pretty(
            run.run_dir
                .join("plans")
                .join(format!("{}.result.json", restored.result_id)),
            &restored,
        )
        .unwrap();
    }
    write_json_pretty(
        run.run_dir.join("health/restore_health_check.json"),
        &healthy_check(),
    )
    .unwrap();
    let mut load = load_artifact_v2(run.run_id.clone(), completed_load("local-target"));
    let control_ref = load_control_ref_override.unwrap_or_else(|| applied_ref.clone());
    attach_load_control_context(&mut load, Some(control_ref), Some(governor.to_string()));
    write_json_pretty(
        run.run_dir
            .join("load")
            .join(format!("cpu.LOAD-RESULT-{governor}.v2.json")),
        &load,
    )
    .unwrap();
    applied_ref
}

fn write_build_info(path: PathBuf, name: &str, version: &str, git_sha: &str) {
    write_json_pretty(
        path,
        &BuildInfo {
            name: name.to_string(),
            version: version.to_string(),
            git_sha: git_sha.to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            build_profile: "test".to_string(),
        },
    )
    .unwrap();
}

fn applied_result(plan: &ControlPlan) -> ControlResult {
    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: plan.plan_id.clone(),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        risk_tier: plan.risk_tier.clone(),
        status: ControlResultStatus::Applied,
        refusal: None,
        restore_lease: None,
        restore_attempted: false,
        restore_result: None,
        time_unix_ms: now_unix_ms(),
    }
}

fn restored_result(plan: &ControlPlan) -> ControlResult {
    ControlResult {
        status: ControlResultStatus::Restored,
        restore_attempted: true,
        ..applied_result(plan)
    }
}

fn healthy_check() -> HealthCheck {
    HealthCheck {
        schema_version: "lab.health_check.v1".to_string(),
        target_id: "local-target".to_string(),
        status: "ok".to_string(),
        inventory_available: true,
        toolchain_available: true,
    }
}

fn completed_load(target_id: &str) -> LoadResult {
    LoadResult {
        schema_version: "lab.load_result.v1".to_string(),
        result_id: "LOAD-RESULT-1".to_string(),
        load_id: "LOAD-1".to_string(),
        target_id: target_id.to_string(),
        status: "completed".to_string(),
        workers: 1,
        duration_ms: 1000,
        abort_reason: None,
        max_observed_temp_c: Some(55.0),
        worker_iterations: vec![1],
        safety_monitor: LoadSafetyMonitorResult {
            sample_interval_ms: 100,
            samples: 10,
            thermal_surface_available: true,
            operator_abort_observed: false,
            restore_on_abort_status: LoadRestoreOnAbortStatus::NotRequired,
        },
        time_unix_ms: now_unix_ms(),
    }
}
