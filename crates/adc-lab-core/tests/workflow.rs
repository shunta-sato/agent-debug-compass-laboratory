use adc_lab_core::*;

#[test]
fn recommendation_is_not_target_measurement_evidence() {
    let artifact = target_operating_contract_workflow_recommendation(WorkflowRecommendationInput {
        run_id: "LAB-RUN-test".to_string(),
        goal: WORKFLOW_PROFILE_SMOKE.to_string(),
        profile_depth: None,
        target: "ssh://target55".to_string(),
        target_id: "target55".to_string(),
        target_class: "raspberry_pi_4".to_string(),
        recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
    })
    .unwrap();

    assert_eq!(artifact.kind, Kind::WorkflowRecommendation);
    assert_eq!(artifact.payload.effective_profile, WORKFLOW_PROFILE_SMOKE);
    assert_eq!(artifact.payload.profile_depth, WorkflowProfileDepth::Smoke);
    assert!(matches!(artifact.status, Status::NotApplicable { .. }));
    assert!(artifact.claims.is_empty());
    assert!(
        !artifact
            .payload
            .evidence_policy
            .recommendation_is_target_measurement_evidence
    );
    assert!(
        !artifact
            .payload
            .evidence_policy
            .raw_primitives_are_claim_producing
    );
}

#[test]
fn codex_agent_instructions_are_registry_derived_without_artifact_selection_heuristics() {
    let artifact = target_operating_contract_workflow_recommendation(WorkflowRecommendationInput {
        run_id: "LAB-RUN-test".to_string(),
        goal: WORKFLOW_PROFILE_SMOKE.to_string(),
        profile_depth: None,
        target: "ssh://target55".to_string(),
        target_id: "target55".to_string(),
        target_class: "raspberry_pi_4".to_string(),
        recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
    })
    .unwrap();
    let text = render_codex_agent_instructions(&artifact, false);

    assert!(text.contains(WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023));
    assert!(text.contains("Do not fall back to a static prompt"));
    assert!(text.contains("stop and report adc-lab version/capability mismatch"));
    assert!(text.contains(COLLECT_PLAN_DEFERRED_NEXT_STEP));
    assert_no_artifact_selection_heuristics(&text, "generated prompt");
}

#[test]
fn collect_plan_steps_are_argv_arrays_and_not_measurement_evidence() {
    let artifact = target_operating_contract_collect_plan(WorkflowCollectPlanInput {
        run_id: "LAB-RUN-test".to_string(),
        goal: WORKFLOW_PROFILE_SMOKE.to_string(),
        profile_depth: None,
        target: "ssh://target55".to_string(),
        target_id: "target55".to_string(),
        target_class: "raspberry_pi_4".to_string(),
        planned_run_dir: "/tmp/adc-lab-run".to_string(),
        collect_plan_path: "/tmp/adc-lab-run/workflows/collect_plan.v2.json".to_string(),
        agent_instructions_path: "/tmp/adc-lab-run/workflows/collect_plan.md".to_string(),
        handoff_dir: "/tmp/handoff".to_string(),
        workflow_recommendation_path: "/tmp/adc-lab-run/workflows/recommendation.v2.json"
            .to_string(),
        workflow_recommendation_ref: None,
        workflow_recommendation_digest: None,
        workload_demand_path: "/tmp/adc-lab-run/inputs/workload_demand.json".to_string(),
        suitability_policy_path: "/tmp/adc-lab-run/inputs/suitability_policy.yaml".to_string(),
        expected_governors: vec!["ondemand".to_string(), "performance".to_string()],
        recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
        network_endpoint: None,
    })
    .unwrap();

    assert_eq!(artifact.kind, Kind::WorkflowCollectPlan);
    assert_eq!(artifact.payload.effective_profile, WORKFLOW_PROFILE_SMOKE);
    assert_eq!(artifact.payload.profile_depth, WorkflowProfileDepth::Smoke);
    assert!(matches!(artifact.status, Status::NotApplicable { .. }));
    assert!(artifact.claims.is_empty());
    assert!(!artifact.payload.packaging_is_target_evidence);
    assert!(artifact.payload.packaging_failure_blocks_handoff);
    assert!(artifact
        .payload
        .source_of_truth_chain
        .contains(&"report.run_validation".to_string()));

    let validation_step = step_by_id(&artifact, "run_validation");
    assert!(validation_step
        .command_argv
        .contains(&"--collect-plan".to_string()));
    assert!(validation_step
        .command_argv
        .contains(&"--include-run".to_string()));
    assert!(validation_step
        .expected_artifact_kinds
        .contains(&"report.run_validation".to_string()));

    let governor_step = step_by_id(&artifact, "governor_sweep_run");
    assert_eq!(governor_step.execution_location, "target_local");
    assert!(!governor_step.requires_controller);
    let guide = artifact
        .payload
        .target_local_execution_guide
        .as_ref()
        .expect("ssh collect plan must include target-local execution guide");
    assert_eq!(guide.applies_to_execution_location, "target_local");
    assert_eq!(
        guide.working_directory_policy,
        "target_local_repository_root"
    );
    assert_eq!(guide.path_prepend[0], "$HOME/.local/bin");
    assert!(guide
        .argv_semantics
        .contains("preserve command_argv as ordered arguments"));
    assert!(guide
        .ssh_invocation_template
        .contains(&"target55".to_string()));
    for category in [
        "command_not_found",
        "path_missing",
        "permission_denied",
        "helper_unavailable",
        "version_skew",
    ] {
        assert!(
            guide
                .failure_diagnostics
                .iter()
                .any(|diagnostic| diagnostic.category == category),
            "missing target-local diagnostic category {category}"
        );
    }
    let target_arg_index = governor_step
        .command_argv
        .iter()
        .position(|arg| arg == "--target")
        .unwrap()
        + 1;
    assert_eq!(governor_step.command_argv[target_arg_index], "local");
    assert!(!governor_step
        .command_argv
        .iter()
        .any(|arg| arg == "ssh://target55"));

    let governor_retrieval_parent_step =
        step_by_id(&artifact, "prepare_target_local_governor_retrieval_parent");
    let reset_governor_retrieval_step = step_by_id(
        &artifact,
        "reset_target_local_governor_retrieval_destination",
    );
    let retrieve_governor_step = step_by_id(&artifact, "retrieve_target_local_governor_sweep");
    let retrieved_governor_dir = "/tmp/adc-lab-run/included/target-local-governor-sweep";
    assert!(
        step_index(&artifact, "governor_sweep_run")
            < step_index(&artifact, "prepare_target_local_governor_retrieval_parent")
    );
    assert!(
        step_index(&artifact, "prepare_target_local_governor_retrieval_parent")
            < step_index(
                &artifact,
                "reset_target_local_governor_retrieval_destination"
            )
    );
    assert!(
        step_index(
            &artifact,
            "reset_target_local_governor_retrieval_destination"
        ) < step_index(&artifact, "retrieve_target_local_governor_sweep")
    );
    assert!(
        step_index(&artifact, "retrieve_target_local_governor_sweep")
            < step_index(&artifact, "run_validation")
    );
    assert_eq!(
        governor_retrieval_parent_step.command_argv,
        vec![
            "mkdir".to_string(),
            "-p".to_string(),
            "/tmp/adc-lab-run/included".to_string()
        ]
    );
    assert_eq!(
        reset_governor_retrieval_step.command_argv,
        vec![
            "rm".to_string(),
            "-rf".to_string(),
            retrieved_governor_dir.to_string()
        ]
    );
    assert_eq!(
        retrieve_governor_step.command_argv,
        vec![
            "scp".to_string(),
            "-r".to_string(),
            "target55:adc-lab-target-local-LAB-RUN-test".to_string(),
            retrieved_governor_dir.to_string()
        ]
    );
    assert!(retrieve_governor_step
        .expected_artifact_paths_or_globs
        .contains(&retrieved_governor_dir.to_string()));
    assert_eq!(
        argv_values(&validation_step.command_argv, "--include-run"),
        vec![retrieved_governor_dir.to_string()]
    );

    let archive_step = step_by_id(&artifact, "archive");
    assert!(archive_step
        .command_argv
        .contains(&"/tmp/handoff/LAB-RUN-test.tgz".to_string()));
    assert!(!archive_step
        .command_argv
        .contains(&"/tmp/adc-lab-run/handoff/adc-lab-run.tgz".to_string()));

    for required_step in [
        "read_only_inventory",
        "toolchain_discover",
        "observe_baseline",
        "cpu_ladder",
        "pressure_probe_set",
        "composite_probe",
        "workload_demand",
    ] {
        step_by_id(&artifact, required_step);
    }

    for step in &artifact.payload.steps {
        assert!(!step.command_argv.is_empty());
        for arg in &step.command_argv {
            for forbidden in ["|", "&&", "$(", "`"] {
                assert!(
                    !arg.contains(forbidden),
                    "argv item must not contain shell fragment {forbidden}: {arg}"
                );
            }
        }
    }

    let instructions = render_collect_plan_agent_instructions(&artifact);
    assert!(instructions.contains("argv: `["));
    assert!(instructions.contains("Target-Local Execution Guide"));
    assert!(instructions.contains("path_prepend: `$HOME/.local/bin"));
    assert!(instructions.contains("quote each remote arg independently"));
    assert!(instructions.contains("permission_denied"));
    assert!(instructions.contains("helper_unavailable"));
    assert!(instructions.contains("version_skew"));
    assert!(instructions.contains("retrieve_target_local_governor_sweep"));
    assert!(instructions.contains("Do not fall back to a static prompt"));
    assert_no_artifact_selection_heuristics(&instructions, "generated collect instructions");
}

fn step_by_id<'a>(
    artifact: &'a Artifact<WorkflowCollectPlanPayload>,
    step_id: &str,
) -> &'a WorkflowCollectPlanStep {
    artifact
        .payload
        .steps
        .iter()
        .find(|step| step.step_id == step_id)
        .unwrap_or_else(|| panic!("collect plan missing full-set skeleton step {step_id}"))
}

fn step_index(artifact: &Artifact<WorkflowCollectPlanPayload>, step_id: &str) -> usize {
    artifact
        .payload
        .steps
        .iter()
        .position(|step| step.step_id == step_id)
        .unwrap_or_else(|| panic!("collect plan missing full-set skeleton step {step_id}"))
}

fn argv_values(argv: &[String], flag: &str) -> Vec<String> {
    argv.windows(2)
        .filter_map(|window| {
            if window[0] == flag {
                Some(window[1].clone())
            } else {
                None
            }
        })
        .collect()
}

fn assert_no_artifact_selection_heuristics(text: &str, label: &str) {
    for forbidden in [
        "PLAN-*.json",
        "APPROVAL-*.json",
        "LEASE-*.json",
        "tail -n 1",
        "ls -t",
        "find ",
        "mtime",
        "newest",
        "latest plan",
        "latest approval",
        "latest lease",
    ] {
        assert!(
            !text.contains(forbidden),
            "{label} must not contain {forbidden}"
        );
    }
}
