use adc_lab_core::{
    canonical_plan_digest, governor_sweep_policy_digest, Actor, ActorKind, ApprovalBounds,
    ApprovalRecord, Artifact, ControlPlan, GovernorSweepPolicyPayload, Kind, RiskTier,
};
use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::io::Read;
use std::net::TcpListener;
#[cfg(unix)]
use std::path::PathBuf;
use std::thread;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn single_plan_path(run_dir: &std::path::Path) -> PathBuf {
    fs::read_dir(run_dir.join("plans"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .unwrap()
}

fn single_approval_path(run_dir: &std::path::Path) -> PathBuf {
    fs::read_dir(run_dir.join("approvals"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .unwrap()
}

fn single_load_artifact_path(run_dir: &std::path::Path, suffix: &str) -> PathBuf {
    fs::read_dir(run_dir.join("loads"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(suffix))
        })
        .unwrap()
}

fn single_v2_load_artifact_path(run_dir: &std::path::Path) -> PathBuf {
    fs::read_dir(run_dir.join("load"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".v2.json"))
        })
        .unwrap()
}

fn write_workload_plan(run_dir: &std::path::Path, adc_lab_bin: &std::path::Path) -> PathBuf {
    let plan_path = run_dir.join("workload.yaml");
    let working_directory = workspace_root();
    let plan = serde_json::json!({
        "schema_version": "lab.workload_run_plan.v1",
        "workload_id": "cli_bounded_smoke",
        "workload_name": "CLI bounded smoke",
        "target": "local",
        "execution": {
            "executable_path": adc_lab_bin.display().to_string(),
            "args": [
                "workload-fixture",
                "bounded-smoke",
                "--duration-ms",
                "800",
                "--memory-bytes",
                "1048576",
                "--storage-bytes",
                "4096"
            ],
            "working_directory": working_directory.display().to_string(),
            "expected_executable_sha256": null,
            "require_executable_sha256": false,
            "reject_setuid": true,
            "reject_world_writable": true,
            "environment_policy": {
                "inherit": false,
                "allowed": [
                    { "name": "PATH", "value": "/usr/bin:/bin:/usr/local/bin" }
                ]
            }
        },
        "bounds": {
            "duration_seconds_max": 3,
            "stdout_bytes_max": 65536,
            "stderr_bytes_max": 65536,
            "memory_bytes_max": 1048576,
            "storage_bytes_max": 4096,
            "thermal_abort_c": null,
            "operator_abort_file": null
        },
        "observation": {
            "sample_interval_ms": 50,
            "process_scoped": true,
            "system_context": true
        },
        "claim_boundary": [
            "exploratory target-local capability evidence only",
            "not production readiness"
        ]
    });
    fs::write(&plan_path, serde_yaml::to_string(&plan).unwrap()).unwrap();
    plan_path
}

#[test]
fn cli_help_mentions_adc_lab() {
    Command::cargo_bin("adc-lab")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(contains("adc-lab"));
}

#[test]
fn workflow_recommend_stdout_is_authority_not_measurement_evidence() {
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "workflow",
            "recommend",
            "--goal",
            "target-operating-contract-fullset",
            "--target",
            "ssh://target55",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "lab.artifact.v2");
    assert_eq!(value["kind"], "workflow.recommendation");
    assert_eq!(value["status"]["state"], "not_applicable");
    assert_eq!(value["claims"].as_array().unwrap().len(), 0);
    assert_eq!(
        value["payload"]["workflow_id"],
        "target-operating-contract-fullset.v0.2.3"
    );
    assert_eq!(
        value["payload"]["evidence_policy"]["recommendation_is_target_measurement_evidence"],
        false
    );
    assert_eq!(
        value["payload"]["evidence_policy"]["raw_primitives_are_claim_producing"],
        false
    );
    assert!(value["payload"]["must_not_use_for_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry.as_str().unwrap().contains("filename order")));
}

#[test]
fn workflow_recommend_run_dir_writes_artifact_and_audit() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "workflow",
            "recommend",
            "--goal",
            "target-operating-contract-fullset",
            "--target",
            "ssh://target55",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("artifact://lab/runs/"));

    let path = temp.path().join("workflows/recommendation.v2.json");
    let recommendation: Artifact<serde_json::Value> =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(recommendation.kind, Kind::WorkflowRecommendation);
    assert!(recommendation.claims.is_empty());

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"workflow.recommend\""));
}

#[test]
fn agent_instructions_generate_codex_prompt_from_workflow_registry() {
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("codex_target55_fullset.md");
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "agent",
            "instructions",
            "--goal",
            "target-operating-contract-fullset",
            "--target",
            "ssh://target55",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--format",
            "codex",
            "--out",
            out.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        summary["workflow_id"],
        "target-operating-contract-fullset.v0.2.3"
    );
    assert_eq!(
        summary["next_step"],
        "run adc-lab collect plan, then follow the emitted argv-array steps"
    );

    let prompt = fs::read_to_string(out).unwrap();
    assert!(prompt.contains("workflow_id: `target-operating-contract-fullset.v0.2.3`"));
    assert!(prompt.contains("adc_lab_version: `"));
    assert!(prompt.contains("reports/run_validation.v2.json"));
    assert!(prompt.contains("reports/target_operating_contract.v2.json"));
    assert!(prompt.contains("Do not fall back to a static prompt or hand-written shell harness"));
    assert!(prompt.contains("stop and report adc-lab version/capability mismatch"));
    assert!(prompt.contains("run adc-lab collect plan"));

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
            !prompt.contains(forbidden),
            "generated prompt must not contain {forbidden}"
        );
    }
}

#[test]
fn collect_plan_writes_v2_argv_steps_and_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let run_dir = temp.path().join("run");
    let out = run_dir.join("workflows/collect_plan.v2.json");
    let instructions_out = run_dir.join("workflows/collect_plan.md");
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "collect",
            "plan",
            "--goal",
            "target-operating-contract-fullset",
            "--target",
            "ssh://target55",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--expected-governors",
            "ondemand,performance",
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
            "--agent-instructions-out",
            instructions_out.to_str().unwrap(),
            "--json",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(summary["artifact_ref"], out.display().to_string());
    assert_eq!(summary["value"]["kind"], "workflow.collect_plan");
    assert_eq!(summary["value"]["status"]["state"], "not_applicable");
    assert_eq!(summary["value"]["claims"].as_array().unwrap().len(), 0);
    assert_eq!(
        summary["value"]["payload"]["workflow_id"],
        "target-operating-contract-fullset.v0.2.3"
    );
    assert_eq!(
        summary["value"]["payload"]["packaging_is_target_evidence"],
        false
    );

    let plan: Artifact<serde_json::Value> =
        serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(plan.kind, Kind::WorkflowCollectPlan);
    let steps = plan.payload["steps"].as_array().unwrap();
    assert!(steps.iter().any(|step| step["step_id"] == "run_validation"));
    assert!(steps.iter().any(|step| step["step_id"] == "archive"));
    for required_step in [
        "read_only_inventory",
        "toolchain_discover",
        "observe_baseline",
        "cpu_ladder",
        "pressure_probe_set",
        "composite_probe",
        "workload_demand",
    ] {
        assert!(
            steps.iter().any(|step| step["step_id"] == required_step),
            "collect plan missing full-set skeleton step {required_step}"
        );
    }
    for step in steps {
        let argv = step["command_argv"].as_array().unwrap();
        assert!(!argv.is_empty());
        for arg in argv {
            let arg = arg.as_str().unwrap();
            for forbidden in ["|", "&&", "$(", "`"] {
                assert!(
                    !arg.contains(forbidden),
                    "argv item must not contain shell fragment {forbidden}: {arg}"
                );
            }
        }
    }

    let governor_step = steps
        .iter()
        .find(|step| step["step_id"] == "governor_sweep_run")
        .unwrap();
    assert_eq!(governor_step["execution_location"], "target_local");
    assert_eq!(governor_step["requires_controller"], false);
    let governor_argv = governor_step["command_argv"].as_array().unwrap();
    let target_index = governor_argv
        .iter()
        .position(|arg| arg == "--target")
        .unwrap()
        + 1;
    assert_eq!(governor_argv[target_index], "local");
    assert!(!governor_argv.iter().any(|arg| arg == "ssh://target55"));

    let validation_step = steps
        .iter()
        .find(|step| step["step_id"] == "run_validation")
        .unwrap();
    assert!(validation_step["command_argv"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "--collect-plan"));
    assert!(validation_step["command_argv"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == "--include-run"));
    assert!(validation_step["expected_artifact_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "report.run_validation"));

    let operating_contract_step = steps
        .iter()
        .find(|step| step["step_id"] == "operating_contract")
        .unwrap();
    let operating_contract_argv = operating_contract_step["command_argv"].as_array().unwrap();
    assert!(operating_contract_argv
        .iter()
        .any(|arg| arg == "--validation"));
    assert!(operating_contract_argv
        .iter()
        .any(|arg| arg == "--strict-fullset"));
    assert!(operating_contract_step["expected_artifact_kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|kind| kind == "report.operating_contract"));

    let archive_step = steps
        .iter()
        .find(|step| step["step_id"] == "archive")
        .unwrap();
    let archive_argv = archive_step["command_argv"].as_array().unwrap();
    let archive_index = archive_argv.iter().position(|arg| arg == "-czf").unwrap() + 1;
    let archive_path = archive_argv[archive_index].as_str().unwrap();
    assert!(archive_path.ends_with("/handoff/run.tgz"));
    assert!(!archive_path.starts_with(run_dir.to_str().unwrap()));

    let instructions = fs::read_to_string(instructions_out).unwrap();
    assert!(instructions.contains("workflow.collect_plan"));
    assert!(instructions.contains("argv: `["));
    assert!(instructions.contains("execution_location: `target_local`"));
    assert!(instructions.contains("Packaging steps are handoff steps, not target evidence"));
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
            !instructions.contains(forbidden),
            "collect instructions must not contain {forbidden}"
        );
    }
}

#[test]
fn workload_run_refuses_ssh_target_with_structured_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let adc_lab_bin = assert_cmd::cargo::cargo_bin("adc-lab");
    let plan_path = write_workload_plan(temp.path(), &adc_lab_bin);

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "workload",
            "run",
            "--target",
            "ssh://target55",
            "--plan",
            plan_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("remote_workload_execution_not_supported_in_v1"));
    let result_path = temp
        .path()
        .join("workloads/cli_bounded_smoke/workload_run_result.json");
    let profile_path = temp.path().join("reports/workload_demand_profile.json");
    assert!(result_path.exists());
    assert!(profile_path.exists());
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(result_path).unwrap()).unwrap();
    assert_eq!(result["status"], "refused");
}

#[test]
fn workload_run_local_captures_process_scoped_demand() {
    let temp = tempfile::tempdir().unwrap();
    let adc_lab_bin = assert_cmd::cargo::cargo_bin("adc-lab");
    let plan_path = write_workload_plan(temp.path(), &adc_lab_bin);

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "workload",
            "run",
            "--target",
            "local",
            "--target-id",
            "target55",
            "--execution-mode",
            "target-local",
            "--plan",
            plan_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("workload_demand_profile.json"));
    let profile_path = temp.path().join("reports/workload_demand_profile.json");
    let profile: serde_json::Value =
        serde_json::from_slice(&fs::read(profile_path).unwrap()).unwrap();
    assert_eq!(profile["target_id"], "target55");
    assert_eq!(profile["execution_mode"], "target_local");
    assert_eq!(profile["demand_scope"], "process_scoped");
    assert!(profile["workload_demand"]["process_cpu_time_ms"]
        .as_f64()
        .is_some_and(|value| value >= 0.0));
    assert_eq!(
        profile["target_conditioned_response"]["portable_between_targets"],
        false
    );
}

#[test]
fn constraints_generate_writes_agent_markdown() {
    let temp = tempfile::tempdir().unwrap();
    let decision_path = temp.path().join("suitability_decision.json");
    let out_path = temp.path().join("constraints.json");
    let md_path = temp.path().join("agent_constraints.md");
    fs::write(
        &decision_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "lab.artifact.v2",
            "kind": "report.suitability",
            "id": "SUITABILITY-001",
            "run_id": "LAB-RUN-001",
            "target_id": "target55",
            "status": { "state": "insufficient" },
            "bounds": null,
            "factors": { "controlled": [], "observed": [], "confounders": [] },
            "metrics": [],
            "claims": [],
            "evidence_refs": [],
            "data_quality": { "level": "partial", "notes": [] },
            "payload": {
                "rule_set_id": "test.suitability",
                "selection_ready": false,
                "workload_id": "workload-001",
                "policy_id": "policy-001",
                "overall_decision": "unknown",
                "dimensions": [],
                "evaluations": [],
                "blocked_claims": ["target.selection.production_ready"],
                "next_evidence": []
            },
            "time_unix_ms": 1
        }))
        .unwrap(),
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "constraints",
            "generate",
            "--decision",
            decision_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--agent-instructions-out",
            md_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"report.constraints\""));
    let markdown = fs::read_to_string(md_path).unwrap();
    assert!(markdown.contains("# Target Constraints"));
    assert!(markdown.contains("## Blocked claims"));
    assert!(markdown.contains("target.selection.production_ready"));
}

#[test]
fn decide_suitability_writes_v2_without_legacy_sidecar() {
    let temp = tempfile::tempdir().unwrap();
    let target_run = temp.path().join("target-run");
    fs::create_dir_all(target_run.join("observations")).unwrap();
    fs::write(
        target_run.join("observations/observe.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "max_observed_temp_c": 61.0,
            "memory_available_kb": 7340032
        }))
        .unwrap(),
    )
    .unwrap();
    let demand_path = temp.path().join("workload_demand_profile.json");
    let policy_path = temp.path().join("policy.yaml");
    let out_path = temp.path().join("suitability_decision.json");
    fs::copy(
        workspace_root().join("tests/golden/lab.workload_demand_profile.v1.valid.json"),
        &demand_path,
    )
    .unwrap();
    fs::copy(
        workspace_root().join("tests/golden/lab.suitability_policy.v1.valid.json"),
        &policy_path,
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            target_run.to_str().unwrap(),
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .assert()
        .success();
    let contract_path = target_run.join("reports/target_operating_contract.v2.json");
    assert!(contract_path.exists());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "decide",
            "suitability",
            "--target-run",
            target_run.to_str().unwrap(),
            "--target-contract",
            contract_path.to_str().unwrap(),
            "--workload-demand",
            demand_path.to_str().unwrap(),
            "--policy",
            policy_path.to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"report.suitability\""));
    let decision: serde_json::Value =
        serde_json::from_slice(&fs::read(&out_path).unwrap()).unwrap();
    assert_eq!(decision["schema"], "lab.artifact.v2");
    assert_eq!(decision["kind"], "report.suitability");
    assert_eq!(decision["payload"]["selection_ready"], true);
    let blocked_claims = decision["payload"]["blocked_claims"].as_array().unwrap();
    assert!(blocked_claims
        .iter()
        .any(|claim| claim == "target.selection.production_ready"));
    assert!(blocked_claims
        .iter()
        .any(|claim| claim == "boundary.thermal_sustained_soak"));
    assert!(!blocked_claims
        .iter()
        .any(|claim| claim.as_str().is_some_and(|claim| claim.contains(' '))));
    assert!(!temp.path().join("suitability_decision.v1.json").exists());
}

#[test]
fn suitability_loop_consumes_tool_produced_v2_artifacts_end_to_end() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("observations")).unwrap();
    fs::write(
        temp.path().join("observations/observe.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "max_observed_temp_c": 61.0,
            "memory_available_kb": 7340032
        }))
        .unwrap(),
    )
    .unwrap();
    let demand_path = temp.path().join("workload_demand_profile.json");
    let policy_path = temp.path().join("policy.yaml");
    let decision_path = temp.path().join("reports/suitability_decision.json");
    let constraints_path = temp.path().join("reports/design_constraint_pack.json");
    let agent_md_path = temp.path().join("reports/agent_constraints.md");
    fs::copy(
        workspace_root().join("tests/golden/lab.workload_demand_profile.v1.valid.json"),
        &demand_path,
    )
    .unwrap();
    fs::copy(
        workspace_root().join("tests/golden/lab.suitability_policy.v1.valid.json"),
        &policy_path,
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .assert()
        .success();
    let contract_path = temp
        .path()
        .join("reports/target_operating_contract.v2.json");
    assert!(contract_path.exists());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "decide",
            "suitability",
            "--target-run",
            temp.path().to_str().unwrap(),
            "--target-contract",
            contract_path.to_str().unwrap(),
            "--workload-demand",
            demand_path.to_str().unwrap(),
            "--policy",
            policy_path.to_str().unwrap(),
            "--out",
            decision_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"report.suitability\""));

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "constraints",
            "generate",
            "--decision",
            decision_path.to_str().unwrap(),
            "--out",
            constraints_path.to_str().unwrap(),
            "--agent-instructions-out",
            agent_md_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"report.constraints\""));

    let decision: serde_json::Value =
        serde_json::from_slice(&fs::read(&decision_path).unwrap()).unwrap();
    assert_eq!(decision["schema"], "lab.artifact.v2");
    assert_eq!(decision["kind"], "report.suitability");
    assert!(decision["payload"]["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim == "target.selection.production_ready"));
    let constraints: serde_json::Value =
        serde_json::from_slice(&fs::read(&constraints_path).unwrap()).unwrap();
    assert_eq!(constraints["schema"], "lab.artifact.v2");
    assert_eq!(constraints["kind"], "report.constraints");
    assert!(constraints["payload"]["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim == "target.selection.production_ready"));
    let markdown = fs::read_to_string(agent_md_path).unwrap();
    assert!(markdown.contains("## Blocked claims"));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.target_operating_contract\""));
    assert!(audit.contains("\"operation\":\"decide.suitability\""));
    assert!(audit.contains("\"operation\":\"constraints.generate\""));
}

#[test]
fn constraints_check_fails_on_blocked_claim_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let pack_path = temp.path().join("constraints.json");
    let claim_path = temp.path().join("CLAIMS.md");
    fs::write(
        &pack_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "lab.artifact.v2",
            "kind": "report.constraints",
            "id": "CONSTRAINTS-001",
            "run_id": "LAB-RUN-001",
            "target_id": "target55",
            "status": { "state": "insufficient" },
            "bounds": null,
            "factors": { "controlled": [], "observed": [], "confounders": [] },
            "metrics": [],
            "claims": [],
            "evidence_refs": [],
            "data_quality": { "level": "partial", "notes": [] },
            "payload": {
                "source_suitability_id": "SUITABILITY-001",
                "workload_id": "workload-001",
                "policy_id": "policy-001",
                "allowed_patterns": [],
                "burst_only_patterns": [],
                "degraded_mode_triggers": [],
                "forbidden_patterns": [],
                "budget_constraints": [],
                "required_runtime_guards": [],
                "blocked_claims": ["target.selection.production_ready"],
                "agent_instructions": [],
                "ci_rules": []
            },
            "time_unix_ms": 1
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(&claim_path, "This target has production readiness.\n").unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "constraints",
            "check",
            "--constraints",
            pack_path.to_str().unwrap(),
            "--path",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"kind\": \"report.constraints_check\""))
        .stdout(contains("\"status\": \"fail\""))
        .stdout(contains("production readiness"));
}

#[test]
fn constraints_check_generated_mode_allows_generated_blocked_claims_section() {
    let temp = tempfile::tempdir().unwrap();
    let pack_path = temp.path().join("constraints.json");
    let generated_md = temp.path().join("agent_constraints.md");
    write_minimal_constraints_pack(&pack_path);
    fs::write(
        &generated_md,
        [
            "# Target Constraints for target55 / workload-001",
            "",
            "Source:",
            "- suitability_artifact: artifact://lab/runs/LAB-RUN-001/reports/suitability.v2.json",
            "",
            "## Must obey",
            "",
            "- Do not claim production readiness from this v2 workload suitability slice.",
            "",
            "## Blocked claims",
            "",
            "- `target.selection.production_ready`: \"production readiness\"",
        ]
        .join("\n"),
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "constraints",
            "check",
            "--constraints",
            pack_path.to_str().unwrap(),
            "--path",
            generated_md.to_str().unwrap(),
            "--mode",
            "generated-constraints",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"report.constraints_check\""))
        .stdout(contains("\"mode\": \"generated_constraints\""))
        .stdout(contains("\"status\": \"pass\""));
}

#[test]
fn constraints_check_generated_mode_fails_on_mismatched_constraints_artifact() {
    let temp = tempfile::tempdir().unwrap();
    let pack_path = temp.path().join("constraints.json");
    let other_pack_path = temp.path().join("other_constraints.json");
    write_minimal_constraints_pack(&pack_path);
    write_minimal_constraints_pack(&other_pack_path);
    let mut other: serde_json::Value =
        serde_json::from_slice(&fs::read(&other_pack_path).unwrap()).unwrap();
    other["id"] = serde_json::json!("CONSTRAINTS-OTHER");
    fs::write(&other_pack_path, serde_json::to_vec_pretty(&other).unwrap()).unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "constraints",
            "check",
            "--constraints",
            pack_path.to_str().unwrap(),
            "--path",
            other_pack_path.to_str().unwrap(),
            "--mode",
            "generated-constraints",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"kind\": \"report.constraints_check\""))
        .stdout(contains("\"mode\": \"generated_constraints\""))
        .stdout(contains("\"status\": \"fail\""))
        .stdout(contains("id mismatch"));
}

#[test]
fn version_commands_emit_build_info_json() {
    assert_build_info("adc-lab");
    assert_build_info("adc-lab-target");
    assert_build_info("adc-lab-priv-helper");
}

fn write_minimal_constraints_pack(path: &std::path::Path) {
    fs::write(
        path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema": "lab.artifact.v2",
            "kind": "report.constraints",
            "id": "CONSTRAINTS-001",
            "run_id": "LAB-RUN-001",
            "target_id": "target55",
            "status": { "state": "insufficient" },
            "bounds": null,
            "factors": { "controlled": [], "observed": [], "confounders": [] },
            "metrics": [],
            "claims": [],
            "evidence_refs": [],
            "data_quality": { "level": "partial", "notes": [] },
            "payload": {
                "source_suitability_id": "SUITABILITY-001",
                "workload_id": "workload-001",
                "policy_id": "policy-001",
                "allowed_patterns": [],
                "burst_only_patterns": [],
                "degraded_mode_triggers": [],
                "forbidden_patterns": [],
                "budget_constraints": [],
                "required_runtime_guards": [],
                "blocked_claims": ["target.selection.production_ready"],
                "agent_instructions": [],
                "ci_rules": []
            },
            "time_unix_ms": 1
        }))
        .unwrap(),
    )
    .unwrap();
}

fn assert_build_info(binary: &str) {
    let output = if binary == "adc-lab" {
        Command::cargo_bin(binary)
            .unwrap()
            .arg("--version")
            .output()
            .unwrap()
    } else {
        Command::new("cargo")
            .args(["run", "-q", "-p", binary, "--", "--version"])
            .output()
            .unwrap()
    };
    assert!(
        output.status.success(),
        "{binary} --version failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["name"], binary);
    for field in ["version", "git_sha", "target_triple", "build_profile"] {
        assert!(
            json[field].as_str().is_some_and(|value| !value.is_empty()),
            "{binary} missing non-empty {field}: {json}"
        );
    }
}

#[test]
fn inventory_local_writes_artifact() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "inventory",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("target_inventory.json"));
    assert!(temp.path().join("inventory/target_inventory.json").exists());
    assert!(temp.path().join("audit.jsonl").exists());
}

#[test]
fn control_apply_audit_records_approval_ref() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success()
        .stdout(contains("\"artifact_ref\""));
    let plan_path = single_plan_path(temp.path());
    let plan: ControlPlan = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    let approval_path = temp.path().join("approval.json");
    fs::write(
        &approval_path,
        serde_json::to_vec_pretty(&matching_approval(&plan)).unwrap(),
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approval",
            approval_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"dry_run_ok\""));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"approval_ref\":\"artifact://lab/runs/"));
    assert!(audit.contains("/approvals/APPROVAL-001.json\""));
}

#[test]
fn control_approve_generates_plan_bound_approval_for_dry_run_apply() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--duration-seconds-max",
            "45",
            "--thermal-celsius-abort",
            "70",
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success()
        .stdout(contains("\"artifact_ref\""));
    let plan_path = single_plan_path(temp.path());
    let plan: ControlPlan = serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "approve",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approved-by",
            "operator",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema_version\": \"lab.approval_record.v1\""));

    let approval_path = single_approval_path(temp.path());
    let approval: ApprovalRecord =
        serde_json::from_slice(&fs::read(&approval_path).unwrap()).unwrap();
    assert_eq!(approval.approved_plan_id, plan.plan_id);
    assert_eq!(
        approval.approved_plan_digest,
        canonical_plan_digest(&plan).unwrap()
    );
    assert_eq!(approval.approved_operation, plan.operation);
    assert_eq!(approval.bounds.duration_seconds_max, 45);
    assert_eq!(approval.bounds.thermal_celsius_abort, Some(70.0));

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approval",
            approval_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"dry_run_ok\""));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"control.approve\""));
    assert!(audit.contains("\"operation\":\"control.apply\""));
    assert!(audit.contains("\"approval_ref\":\"artifact://lab/runs/"));
}

#[test]
fn control_approve_refuses_remote_target_plan() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "ssh://pi4-demo",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success();
    let plan_path = single_plan_path(temp.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "approve",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approved-by",
            "operator",
        ])
        .assert()
        .failure()
        .stderr(contains("control approval is local-target only"));
    let approval_files = fs::read_dir(temp.path().join("approvals"))
        .unwrap()
        .flatten()
        .count();
    assert_eq!(approval_files, 0);
}

#[test]
fn report_validate_run_writes_artifact_gaps_and_fails_closed_for_non_measured() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success();
    let plan_path = single_plan_path(temp.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "approve",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approved-by",
            "operator",
        ])
        .assert()
        .success();
    let approval_path = single_approval_path(temp.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approval",
            approval_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "validate-run",
            "--run",
            temp.path().to_str().unwrap(),
            "--expected-governors",
            "performance",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"kind\": \"report.run_validation\""))
        .stderr(contains("non-measured governor evidence"));

    let validation_path = temp.path().join("reports/run_validation.v2.json");
    let gaps_path = temp.path().join("reports/GAPS.md");
    assert!(validation_path.exists());
    assert!(gaps_path.exists());
    let validation: serde_json::Value =
        serde_json::from_slice(&fs::read(validation_path).unwrap()).unwrap();
    assert_eq!(validation["kind"], "report.run_validation");
    assert_eq!(
        validation["payload"]["overall_validity"],
        serde_json::json!("insufficient")
    );
    assert_eq!(
        validation["payload"]["governor_results"][0]["validity"],
        serde_json::json!("insufficient")
    );
    let gaps = fs::read_to_string(gaps_path).unwrap();
    assert!(gaps.contains("performance"));
    assert!(gaps.contains("insufficient"));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.validate_run\""));
}

#[test]
fn report_validate_run_accepts_include_run_and_records_run_set_identity() {
    let primary = tempfile::tempdir().unwrap();
    let included = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "local",
            "--run-dir",
            included.path().to_str().unwrap(),
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success();
    let plan_path = single_plan_path(included.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "approve",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approved-by",
            "operator",
        ])
        .assert()
        .success();
    let approval_path = single_approval_path(included.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--approval",
            approval_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "validate-run",
            "--run",
            primary.path().to_str().unwrap(),
            "--include-run",
            included.path().to_str().unwrap(),
            "--expected-governors",
            "performance",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--allow-non-measured",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"report.run_validation\""));

    let validation_path = primary.path().join("reports/run_validation.v2.json");
    let validation: serde_json::Value =
        serde_json::from_slice(&fs::read(validation_path).unwrap()).unwrap();
    assert_eq!(
        validation["payload"]["target_id"],
        serde_json::json!("target55")
    );
    assert_eq!(
        validation["payload"]["target_class"],
        serde_json::json!("raspberry_pi_4")
    );
    assert_eq!(
        validation["payload"]["included_run_refs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert!(validation["payload"]["subject_run_set_id"]
        .as_str()
        .unwrap()
        .starts_with("RUN-SET-"));
    assert!(validation["payload"]["governor_results"][0]["plan_ref"]
        .as_str()
        .unwrap()
        .contains("artifact://lab/runs/"));
    assert_eq!(
        validation["payload"]["governor_results"][0]["validity"],
        serde_json::json!("insufficient")
    );
}

#[test]
fn governor_sweep_prepare_approve_and_dry_run_writes_validation() {
    let temp = tempfile::tempdir().unwrap();
    let request_path = temp.path().join("approvals/governor_sweep_request.v2.json");
    let policy_path = temp.path().join("approvals/governor_sweep_policy.v2.json");

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "prepare",
            "--target",
            "local",
            "--governors",
            "performance,powersave",
            "--duration-seconds-max",
            "45",
            "--thermal-celsius-abort",
            "70",
            "--requested-by",
            "codex",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--out",
            request_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"control.governor_sweep_policy\""))
        .stdout(contains("\"policy_state\": \"requested\""));
    let request: serde_json::Value =
        serde_json::from_slice(&fs::read(&request_path).unwrap()).unwrap();
    assert_eq!(request["payload"]["policy_state"], "requested");

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "run",
            "--target",
            "local",
            "--governors",
            "performance",
            "--approval-policy",
            request_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("approved sweep policy"));

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "approve",
            "--request",
            request_path.to_str().unwrap(),
            "--approved-by",
            "operator",
            "--out",
            policy_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"policy_state\": \"approved\""));
    let policy: serde_json::Value =
        serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    assert_eq!(policy["payload"]["policy_state"], "approved");
    assert_eq!(policy["payload"]["approved_by"]["id"], "operator");

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "run",
            "--target",
            "local",
            "--governors",
            "performance",
            "--approval-policy",
            policy_path.to_str().unwrap(),
            "--duration-seconds-max",
            "45",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--dry-run",
            "--allow-non-measured",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"report.run_validation\""));

    assert!(temp.path().join("reports/run_validation.v2.json").exists());
    assert!(temp.path().join("reports/GAPS.md").exists());
    let plan_files = fs::read_dir(temp.path().join("plans"))
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".json"))
        })
        .count();
    assert_eq!(plan_files, 2);
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"control.governor_sweep.prepare\""));
    assert!(audit.contains("\"operation\":\"control.governor_sweep.approve\""));
    assert!(audit.contains("\"operation\":\"control.governor_sweep\""));
}

#[test]
fn governor_sweep_policy_scope_and_digest_mismatch_refuse_before_plans() {
    let temp = tempfile::tempdir().unwrap();
    let request_path = temp.path().join("approvals/governor_sweep_request.v2.json");
    let policy_path = temp.path().join("approvals/governor_sweep_policy.v2.json");

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "prepare",
            "--target",
            "local",
            "--governors",
            "performance",
            "--duration-seconds-max",
            "45",
            "--requested-by",
            "codex",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--out",
            request_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "approve",
            "--request",
            request_path.to_str().unwrap(),
            "--approved-by",
            "operator",
            "--out",
            policy_path.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "run",
            "--target",
            "local",
            "--governors",
            "powersave",
            "--approval-policy",
            policy_path.to_str().unwrap(),
            "--duration-seconds-max",
            "45",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("does not include governor powersave"));
    assert_eq!(fs::read_dir(temp.path().join("plans")).unwrap().count(), 0);

    let mut policy: Artifact<GovernorSweepPolicyPayload> =
        serde_json::from_slice(&fs::read(&policy_path).unwrap()).unwrap();
    policy.payload.governors.push("powersave".to_string());
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "run",
            "--target",
            "local",
            "--governors",
            "powersave",
            "--approval-policy",
            policy_path.to_str().unwrap(),
            "--duration-seconds-max",
            "45",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("policy digest mismatch"));
    assert_eq!(fs::read_dir(temp.path().join("plans")).unwrap().count(), 0);

    policy.payload.policy_digest = governor_sweep_policy_digest(&policy.payload).unwrap();
    policy.payload.expires_at_unix_ms = 1;
    policy.payload.policy_digest = governor_sweep_policy_digest(&policy.payload).unwrap();
    fs::write(&policy_path, serde_json::to_vec_pretty(&policy).unwrap()).unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "governor-sweep",
            "run",
            "--target",
            "local",
            "--governors",
            "powersave",
            "--approval-policy",
            policy_path.to_str().unwrap(),
            "--duration-seconds-max",
            "45",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--dry-run",
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("policy is expired"));
    assert_eq!(fs::read_dir(temp.path().join("plans")).unwrap().count(), 0);
}

#[test]
fn restore_dry_run_does_not_write_restore_health_check() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = workspace_root().join("tests/golden/lab.restore_lease.v1.valid.json");
    let mut lease: serde_json::Value = serde_json::from_slice(&fs::read(fixture).unwrap()).unwrap();
    lease["target_id"] = serde_json::json!("local-target");
    let lease_path = temp.path().join("local-lease.json");
    fs::write(&lease_path, serde_json::to_vec_pretty(&lease).unwrap()).unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "restore",
            "--lease",
            lease_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"dry_run_ok\""));
    assert!(!temp
        .path()
        .join("health/restore_health_check.json")
        .exists());
}

#[test]
fn load_cpu_operator_abort_records_safety_monitor_without_abort_path() {
    let temp = tempfile::tempdir().unwrap();
    let abort_file = temp.path().join("operator-abort");
    fs::write(&abort_file, b"abort").unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "load",
            "cpu",
            "--target",
            "local",
            "--workers",
            "1",
            "--duration",
            "3s",
            "--operator-abort-file",
            abort_file.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"load\""))
        .stdout(contains("\"status\": \"aborted\""))
        .stdout(contains("\"abort_reason\": \"operator_abort\""));

    let abort_path_text = abort_file.to_str().unwrap();
    let plan_path = single_load_artifact_path(temp.path(), ".plan.json");
    let result_path = single_v2_load_artifact_path(temp.path());
    let plan_text = fs::read_to_string(&plan_path).unwrap();
    let result_text = fs::read_to_string(&result_path).unwrap();
    assert!(!plan_text.contains(abort_path_text));
    assert!(!result_text.contains(abort_path_text));
    assert!(fs::read_dir(temp.path().join("loads"))
        .unwrap()
        .flatten()
        .all(|entry| !entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".result.json"))));

    let plan: serde_json::Value = serde_json::from_str(&plan_text).unwrap();
    assert_eq!(
        plan["safety_monitor"]["operator_abort_enabled"],
        serde_json::json!(true)
    );
    assert_eq!(
        plan["safety_monitor"]["restore_on_abort"],
        serde_json::json!("not_required")
    );

    let result: serde_json::Value = serde_json::from_str(&result_text).unwrap();
    assert_eq!(result["schema"], "lab.artifact.v2");
    assert_eq!(result["kind"], "load");
    assert_eq!(result["payload"]["status"], "aborted");
    assert_eq!(result["payload"]["abort_reason"], "operator_abort");
    assert_eq!(
        result["payload"]["operator_abort_observed"],
        serde_json::json!(true)
    );
    assert_eq!(
        result["payload"]["restore_on_abort_status"],
        serde_json::json!("not_required")
    );

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"load.cpu\""));
    assert!(audit.contains("\"result\":\"aborted\""));
}

#[test]
fn pressure_run_local_writes_typed_artifact_and_audit() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "latency_jitter",
            "--duration",
            "1ms",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"pressure\""))
        .stdout(contains("\"pressure_kind\": \"latency_jitter\""));

    let pressure_files = fs::read_dir(temp.path().join("pressure"))
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".v2.json"))
        })
        .count();
    assert_eq!(pressure_files, 1);
    assert!(fs::read_dir(temp.path().join("pressure"))
        .unwrap()
        .flatten()
        .all(|entry| !entry
            .path()
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".result.json"))));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"pressure.run\""));
}

#[test]
fn pressure_network_bounded_transfer_records_generated_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = listener.local_addr().unwrap().to_string();
    let receiver = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut buffer = [0u8; 4096];
        let mut received = 0usize;
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            received += read;
        }
        received
    });

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "network_io",
            "--duration",
            "1ms",
            "--network-endpoint",
            &endpoint,
            "--network-bytes",
            "4096",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"network_mode\": \"bounded_transfer\""))
        .stdout(contains("\"network_traffic_generated_bytes\": 4096"));

    assert_eq!(receiver.join().unwrap(), 4096);

    let pressure_path = fs::read_dir(temp.path().join("pressure"))
        .unwrap()
        .flatten()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains("network_io") && name.ends_with(".v2.json"))
        })
        .unwrap();
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(pressure_path).unwrap()).unwrap();
    assert_eq!(result["schema"], "lab.artifact.v2");
    assert_eq!(result["kind"], "pressure");
    assert_eq!(result["status"]["state"], "measured_partial");
    assert_eq!(result["payload"]["evidence_class"], "boundary_probe");
    assert_eq!(result["payload"]["network_mode"], "bounded_transfer");
    assert_eq!(result["payload"]["network_traffic_generated_bytes"], 4096);
}

#[test]
fn report_operating_contract_writes_contract_artifacts() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "latency_jitter",
            "--duration",
            "1ms",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("target_operating_contract_ref"))
        .stdout(contains("\"validation_gate\""))
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"report.operating_contract\""));

    assert!(temp
        .path()
        .join("reports/target_operating_contract.v2.json")
        .exists());
    let contract: serde_json::Value = serde_json::from_slice(
        &fs::read(
            temp.path()
                .join("reports/target_operating_contract.v2.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let production_ready = contract["payload"]["evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|evaluation| {
            evaluation["rule_id"] == "operating.production_readiness_requires_run_report"
        })
        .unwrap();
    assert_eq!(production_ready["decision"], "blocked");
    assert!(production_ready["missing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "matching_report.run_validation"));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.target_operating_contract\""));
}

#[test]
fn report_operating_contract_strict_fullset_fails_after_writing_when_validation_missing() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--target-class",
            "raspberry_pi_4",
            "--strict-fullset",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"validation_gate\""))
        .stdout(contains("target_operating_contract_ref"))
        .stderr(contains("no --validation artifact was provided"));

    assert!(temp
        .path()
        .join("reports/target_operating_contract.v2.json")
        .exists());
}

#[test]
fn report_operating_contract_keeps_production_ready_blocked_after_measured_validation() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "pack",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "target55",
            "--target",
            "local",
            "--json",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "validate-run",
            "--run",
            temp.path().to_str().unwrap(),
            "--expected-governors",
            "performance",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--allow-non-measured",
            "--json",
        ])
        .assert()
        .success();

    let validation_path = temp.path().join("reports/run_validation.v2.json");
    let mut validation: serde_json::Value =
        serde_json::from_slice(&fs::read(&validation_path).unwrap()).unwrap();
    validation["status"] = serde_json::json!({"state": "measured"});
    validation["data_quality"]["level"] = serde_json::json!("complete");
    validation["payload"]["overall_validity"] = serde_json::json!("measured");
    validation["payload"]["governor_results"][0]["validity"] = serde_json::json!("measured");
    validation["payload"]["governor_results"][0]["messages"] = serde_json::json!([]);
    validation["payload"]["governor_results"][0]["next_evidence"] = serde_json::json!([]);
    validation["payload"]["gaps"] = serde_json::json!([]);
    fs::write(
        &validation_path,
        serde_json::to_vec_pretty(&validation).unwrap(),
    )
    .unwrap();

    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            temp.path().to_str().unwrap(),
            "--validation",
            validation_path.to_str().unwrap(),
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--strict-fullset",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"validation_gate\""))
        .get_output()
        .stdout
        .clone();

    let summary: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(summary["validation_gate"]["measured"], true);
    let contract: serde_json::Value = serde_json::from_slice(
        &fs::read(
            temp.path()
                .join("reports/target_operating_contract.v2.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let production_ready = contract["payload"]["evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|evaluation| {
            evaluation["rule_id"] == "operating.production_readiness_requires_run_report"
        })
        .unwrap();
    assert_eq!(production_ready["decision"], "blocked");
    let missing = production_ready["missing"].as_array().unwrap();
    assert!(!missing
        .iter()
        .any(|item| item == "matching_report.run_validation"));
    assert!(missing
        .iter()
        .any(|item| item == "production_operating_envelope"));
    assert!(production_ready["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item
            .as_str()
            .unwrap()
            .ends_with("reports/run_validation.v2.json")));
}

#[test]
fn report_operating_contract_rejects_validation_copied_from_another_run_set() {
    let source = tempfile::tempdir().unwrap();
    let target = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "validate-run",
            "--run",
            source.path().to_str().unwrap(),
            "--expected-governors",
            "performance",
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--allow-non-measured",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"kind\": \"report.run_validation\""));

    let copied_validation = target.path().join("reports/copied_run_validation.v2.json");
    fs::create_dir_all(copied_validation.parent().unwrap()).unwrap();
    fs::copy(
        source.path().join("reports/run_validation.v2.json"),
        &copied_validation,
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            target.path().to_str().unwrap(),
            "--validation",
            copied_validation.to_str().unwrap(),
            "--target-id",
            "target55",
            "--target-class",
            "raspberry_pi_4",
            "--strict-fullset",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(contains("\"validation_gate\""))
        .stderr(contains(
            "subject_run_set_id does not match current run set",
        ));

    let contract: serde_json::Value = serde_json::from_slice(
        &fs::read(
            target
                .path()
                .join("reports/target_operating_contract.v2.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let production_ready = contract["payload"]["evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|evaluation| {
            evaluation["rule_id"] == "operating.production_readiness_requires_run_report"
        })
        .unwrap();
    assert_eq!(production_ready["decision"], "blocked");
}

#[test]
fn report_operating_contract_accepts_include_run_in_v2_store() {
    let primary = tempfile::tempdir().unwrap();
    let included = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "latency_jitter",
            "--duration",
            "1ms",
            "--run-dir",
            primary.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "observer_pressure",
            "--duration",
            "1ms",
            "--run-dir",
            included.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            primary.path().to_str().unwrap(),
            "--include-run",
            included.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"included_run_count\": 1"))
        .stdout(contains("\"kind\": \"report.operating_contract\""));

    assert!(primary
        .path()
        .join("reports/target_operating_contract.v2.json")
        .exists());
    assert!(!primary
        .path()
        .join("reports/run_set_manifest.json")
        .exists());
    assert!(!primary
        .path()
        .join("reports/multi_run_operating_contract.json")
        .exists());

    let audit = fs::read_to_string(primary.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.target_operating_contract\""));
}

#[test]
fn pressure_composite_smoke_does_not_support_coupling_without_measured_effect() {
    let temp = tempfile::tempdir().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "composite",
            "--target",
            "local",
            "--scenario",
            "memory_storage_jitter",
            "--duration",
            "1ms",
            "--memory-bytes",
            "1048576",
            "--storage-bytes",
            "4096",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"schema\": \"lab.artifact.v2\""))
        .stdout(contains("\"kind\": \"composite\""))
        .stdout(contains("memory_storage_jitter"));

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "pressure",
            "run",
            "--target",
            "local",
            "--kind",
            "memory_pressure",
            "--duration",
            "1ms",
            "--memory-bytes",
            "1048576",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-contract",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--target-class",
            "raspberry_pi_4",
            "--json",
        ])
        .assert()
        .success();

    let composite_dir = temp.path().join("composite");
    assert!(fs::read_dir(&composite_dir)
        .unwrap()
        .flatten()
        .any(|entry| entry
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains("memory_storage_jitter")));

    let contract: serde_json::Value = serde_json::from_slice(
        &fs::read(
            temp.path()
                .join("reports/target_operating_contract.v2.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(contract["payload"]["evaluations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|evaluation| evaluation["rule_id"]
            == "operating.memory_storage_coupling_requires_composite"
            && evaluation["matched"] == false
            && evaluation["decision"] == "blocked"));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"pressure.composite\""));
}

fn matching_approval(plan: &ControlPlan) -> ApprovalRecord {
    ApprovalRecord {
        schema_version: "lab.approval_record.v1".to_string(),
        approval_id: "APPROVAL-001".to_string(),
        target_id: plan.target_id.clone(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        operation_summary: "Set CPU governor".to_string(),
        approved_plan_id: plan.plan_id.clone(),
        approved_plan_digest: canonical_plan_digest(plan).unwrap(),
        approved_operation: plan.operation.clone(),
        approved_by: Actor {
            kind: ActorKind::Human,
            id: "operator".to_string(),
        },
        bounds: ApprovalBounds {
            duration_seconds_max: plan.bounds.duration_seconds_max,
            thermal_celsius_abort: plan.bounds.thermal_celsius_abort,
        },
        restore_required: true,
        approved_actions: vec![plan.operation.operation_id.clone()],
        time_unix_ms: 1,
    }
}

#[test]
fn experiment_dry_run_and_report_pack_work() {
    let temp = tempfile::tempdir().unwrap();
    let matrix = workspace_root().join("examples/experiments/pi4_cpu_governor_smoke.yaml");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "experiment",
            "run",
            "--target",
            "local",
            "--matrix",
            matrix.to_str().unwrap(),
            "--dry-run",
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("experiment_run.json"));

    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "pack",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
        ])
        .assert()
        .success()
        .stdout(contains("run_report.v2.json"))
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["value"]["kind"], "report.run");
    let refs = value["value"]["payload"]["artifact_refs"]
        .as_array()
        .unwrap();
    assert!(refs.iter().all(|artifact| artifact
        .as_str()
        .unwrap()
        .starts_with("artifact://lab/runs/")));
    assert!(refs.iter().all(|artifact| !artifact
        .as_str()
        .unwrap()
        .contains(temp.path().to_str().unwrap())));
}

#[test]
fn experiment_real_run_executes_supported_bounded_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let matrix = workspace_root().join("examples/experiments/bounded_load_observe_smoke.yaml");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "experiment",
            "run",
            "--target",
            "local",
            "--matrix",
            matrix.to_str().unwrap(),
            "--trial-load-duration",
            "1s",
            "--trial-observe-duration",
            "0s",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("experiment_run.json"));

    let run: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("experiments/experiment_run.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(run["dry_run"], false);
    let trials = run["trials"].as_array().unwrap();
    assert_eq!(trials.len(), 2);
    assert!(trials.iter().all(|trial| trial["status"] == "completed"));
    assert!(trials.iter().all(
        |trial| trial["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .all(|artifact| artifact
                .as_str()
                .unwrap()
                .starts_with("artifact://lab/runs/"))
    ));
    assert!(trials.iter().any(
        |trial| trial["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                let artifact = artifact.as_str().unwrap();
                artifact.contains("/load/experiments/") && artifact.ends_with(".v2.json")
            })
    ));

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.path().join("reports/run_report.v2.json")).unwrap())
            .unwrap();
    assert_eq!(report["kind"], "report.run");
    assert!(report["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "supported"
            && claim["claim_id"] == "run.experiment_bounded_matrix_executed"
    }));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert_eq!(
        audit.matches("\"operation\":\"experiment.trial\"").count(),
        2
    );
    assert!(audit.contains("\"operation\":\"experiment.run\""));
    assert!(audit.contains("\"result\":\"completed\""));
}

#[test]
fn experiment_real_run_blocks_unsupported_controlled_factor() {
    let temp = tempfile::tempdir().unwrap();
    let matrix_path = temp.path().join("unsupported-governor.yaml");
    fs::write(
        &matrix_path,
        r#"schema_version: lab.experiment_matrix.v1
matrix_id: MATRIX-UNSUPPORTED-GOVERNOR
description: Unsupported governor factor remains blocked by PR6 runner.
factors:
  - name: governor
    kind: controlled_factor
    levels:
      - performance
warmup_seconds: 0
cooldown_seconds: 0
repetitions: 1
order: listed
"#,
    )
    .unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "experiment",
            "run",
            "--target",
            "local",
            "--matrix",
            matrix_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("experiment_run.json"));

    let run: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("experiments/experiment_run.json")).unwrap(),
    )
    .unwrap();
    let trial = &run["trials"].as_array().unwrap()[0];
    assert_eq!(trial["status"], "blocked");
    assert!(trial["failure"]
        .as_str()
        .unwrap()
        .contains("controlled factor 'governor' is not supported"));
    assert!(trial["artifact_refs"].as_array().unwrap().is_empty());

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.path().join("reports/run_report.v2.json")).unwrap())
            .unwrap();
    assert_eq!(report["kind"], "report.run");
    assert!(report["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "blocked"
            && claim["claim_id"] == "run.experiment_bounded_matrix_executed"
    }));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"experiment.trial\""));
    assert!(audit.contains("\"operation\":\"experiment.run\""));
    assert!(audit.contains("\"result\":\"blocked\""));
}

#[test]
fn report_operating_point_marks_read_only_run_observational_only() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "familiarize",
            "read-only",
            "--target",
            "local",
            "--duration",
            "0s",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-point",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("run_report.v2.json"))
        .stdout(contains("\"coverage_status\": \"observational_only\""))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let coverage = &value["value"]["payload"]["operating_point"];
    assert_eq!(coverage["coverage_status"], "observational_only");
    assert!(coverage["observed_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "default_policy_frequency"
            && point["level"] == "observed_current_policy"));
    assert!(coverage["blocked_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "fixed_cpu_frequency"
            && point["coverage_status"] == "not_controllable"));
    assert!(value["value"]["claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim["decision"] == "blocked"
            && claim["claim_id"] == "operating_point.fixed_cpu_frequency_verified"));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.run\""));
}

#[test]
fn privilege_provider_status_records_option_b_disabled_and_audit() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "privilege",
            "provider-status",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("privilege_provider_status.json"))
        .stdout(contains("\"active_provider_id\": \"option_a_sudo_helper\""))
        .stdout(contains("\"availability\": \"planned_disabled\""))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let artifact_ref = value["artifact_ref"].as_str().unwrap();
    assert!(artifact_ref.starts_with("artifact://lab/runs/"));
    assert!(!artifact_ref.contains(temp.path().to_str().unwrap()));

    let path = temp.path().join("privilege/privilege_provider_status.json");
    assert!(path.exists());
    let status: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(status["schema_version"], "lab.privilege_provider_status.v1");
    assert!(status["providers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|provider| {
            provider["provider_id"] == "option_b_systemd_unix_socket"
                && provider["availability"] == "planned_disabled"
                && provider["default_enabled"] == false
                && provider["operations_allowed"]
                    .as_array()
                    .unwrap()
                    .is_empty()
        }));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"privilege.provider_status\""));
    assert!(audit.contains("\"result\":\"recorded\""));
}

#[test]
fn privilege_doctor_writes_noninteractive_readiness_report() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "privilege",
            "doctor",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("privilege_doctor.json"))
        .stdout(contains("\"schema_version\": \"lab.privilege_doctor.v1\""));

    let path = temp.path().join("privilege/privilege_doctor.json");
    assert!(path.exists());
    let report: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(report["schema_version"], "lab.privilege_doctor.v1");
    assert!(report["sudo_non_interactive_available"].is_boolean());
    assert!(report["next_action"]
        .as_str()
        .is_some_and(|value| !value.is_empty()));
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|check| check["check_id"] == "sudo.non_interactive"));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"privilege.doctor\""));
}

#[test]
fn privilege_setup_plans_are_instruction_only_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "privilege",
            "install-plan",
            "--target",
            "local",
            "--helper-bin",
            "./target/release/adc-lab-priv-helper",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("privilege_install_plan.json"))
        .stdout(contains("\"plan_kind\": \"install\""));
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "privilege",
            "uninstall-plan",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("privilege_uninstall_plan.json"))
        .stdout(contains("\"plan_kind\": \"uninstall\""));

    let install: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("privilege/privilege_install_plan.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(install["schema_version"], "lab.privilege_setup_plan.v1");
    assert!(install["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|command| command.as_str().unwrap().contains("sudo install")));

    let uninstall: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("privilege/privilege_uninstall_plan.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(uninstall["schema_version"], "lab.privilege_setup_plan.v1");
    assert!(uninstall["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|warning| warning.as_str().unwrap().contains("instructions only")));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"privilege.install_plan\""));
    assert!(audit.contains("\"operation\":\"privilege.uninstall_plan\""));
    assert!(audit.contains("\"result\":\"instruction_only\""));
}

#[test]
fn report_operating_point_marks_bounded_matrix_controlled_subset() {
    let temp = tempfile::tempdir().unwrap();
    let matrix = workspace_root().join("examples/experiments/bounded_load_observe_smoke.yaml");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "experiment",
            "run",
            "--target",
            "local",
            "--matrix",
            matrix.to_str().unwrap(),
            "--trial-load-duration",
            "1s",
            "--trial-observe-duration",
            "0s",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "operating-point",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"coverage_status\": \"controlled_subset\""))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let coverage = &value["value"]["payload"]["operating_point"];
    assert_eq!(coverage["coverage_status"], "controlled_subset");
    assert!(coverage["controlled_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "cpu_load_workers"
            && !point["evidence_refs"].as_array().unwrap().is_empty()));
    assert!(value["value"]["claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim["decision"] == "supported"
            && claim["claim_id"] == "operating_point.bounded_workload_measured"));
    assert!(value["value"]["claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim["decision"] == "blocked"
            && claim["claim_id"] == "operating_point.fixed_cpu_frequency_verified"));
    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.run\""));
}

#[test]
fn familiarize_read_only_writes_manifest_run_report_and_audit() {
    let temp = tempfile::tempdir().unwrap();
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "familiarize",
            "read-only",
            "--target",
            "local",
            "--duration",
            "0s",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"report_status\": \"observational_read_only\""))
        .stdout(contains("\"run_manifest_ref\""))
        .stdout(contains("\"run_report_ref\""))
        .stdout(contains("run.tool_qualification_summary_generated"))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(value["run_manifest_ref"]
        .as_str()
        .unwrap()
        .starts_with("artifact://lab/runs/"));
    assert!(value["run_report_ref"]
        .as_str()
        .unwrap()
        .starts_with("artifact://lab/runs/"));
    assert_eq!(value["value"]["kind"], "report.run");

    assert!(temp.path().join("run_manifest.json").exists());
    assert!(temp.path().join("reports/run_report.v2.json").exists());
    let report_files = fs::read_dir(temp.path().join("reports"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(report_files, vec!["run_report.v2.json"]);
    assert!(temp.path().join("inventory/target_inventory.json").exists());
    assert!(temp
        .path()
        .join("toolchain/toolchain_inventory.json")
        .exists());
    assert!(temp
        .path()
        .join("tools/tool_qualification_summary.json")
        .exists());
    assert!(temp.path().join("observations/observe.json").exists());

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.path().join("run_manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest["mode"], "read_only_familiarization");
    assert_eq!(
        manifest["run_id"], value["run_id"],
        "manifest and command output must share the same logical run_id"
    );
    assert_eq!(manifest["operations_summary"]["inventory"], "completed");
    assert_eq!(
        manifest["operations_summary"]["toolchain_discovery"],
        "completed"
    );
    assert_eq!(
        manifest["operations_summary"]["passive_observe"],
        "completed"
    );
    assert_eq!(manifest["operations_summary"]["bounded_load"], "not_run");
    assert_eq!(manifest["claim_trace_ref"], value["run_report_ref"]);
    assert!(!manifest["adc_lab_version"].as_str().unwrap().is_empty());
    assert!(!manifest["adc_lab_git_sha"].as_str().unwrap().is_empty());
    assert!(!manifest["adc_lab_target_version"]
        .as_str()
        .unwrap()
        .is_empty());
    assert!(manifest["release_tag"].as_str().unwrap().starts_with('v'));
    assert!(manifest["binary_sha256"]["adc-lab"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert!(manifest["data_quality"]["missing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "controlled operating point experiment was not run"));

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(temp.path().join("reports/run_report.v2.json")).unwrap())
            .unwrap();
    assert!(report["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "supported"
            && claim["claim_id"] == "run.tool_qualification_summary_generated"
    }));
    assert!(report["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "blocked"
            && claim["claim_id"] == "operating_point.fixed_cpu_frequency_verified"
    }));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    for operation in [
        "inventory",
        "toolchain.discover",
        "observe",
        "tool.qualify_inventory",
        "tool.version",
        "evidence.write",
        "report.run",
        "run_manifest.write",
    ] {
        assert!(audit.contains(&format!("\"operation\":\"{operation}\"")));
    }
}

#[test]
fn tool_qualify_inventory_accepts_builtin_readonly_and_bounded_load_tools() {
    let temp = tempfile::tempdir().unwrap();
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "toolchain",
            "discover",
            "--target",
            "local",
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let inventory_path = temp.path().join("toolchain/toolchain_inventory.json");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "tool",
            "qualify-inventory",
            "--inventory",
            inventory_path.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("tool_qualification_summary.json"));

    let summary: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("tools/tool_qualification_summary.json")).unwrap(),
    )
    .unwrap();
    assert!(summary["evidence_accepted_tool_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool == "linux.procfs"));
    assert!(summary["evidence_accepted_tool_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool == "adc-lab-builtin-cpu-load"));
    assert!(summary["evidence_rejected_tool_ids"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool == "linux.cpufreq.sysfs"));

    let procfs_report: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("tools/linux-procfs.qualification.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(procfs_report["status"], "builtin");
    assert_eq!(procfs_report["evidence_accepted"], true);

    let load_report: serde_json::Value = serde_json::from_slice(
        &fs::read(
            temp.path()
                .join("tools/adc-lab-builtin-cpu-load.qualification.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(load_report["evidence_accepted"], true);
    assert!(load_report["reason"]
        .as_str()
        .unwrap()
        .contains("builtin bounded CPU load"));
    assert!(load_report["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item.as_str().unwrap().contains("production readiness")));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"tool.qualify_inventory\""));
}

#[test]
fn tool_qualification_records_unqualified_agent_tool() {
    let temp = tempfile::tempdir().unwrap();
    let manifest = workspace_root().join("examples/tools/linux_cpufreq_reader.yaml");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "tool",
            "qualify",
            "--manifest",
            manifest.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"agent_created_unqualified\""))
        .stdout(contains("\"evidence_accepted\": false"));
}

#[test]
fn tool_qualification_accepts_complete_agent_observation_adapter_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let evidence_dir = temp.path().join("evidence-inputs");
    fs::create_dir_all(&evidence_dir).unwrap();
    let output_schema = evidence_dir.join("output-schema.json");
    let dry_run = evidence_dir.join("dry-run.json");
    let manual_comparison = evidence_dir.join("manual-comparison.json");
    let static_safety_review = evidence_dir.join("static-safety-review.txt");
    fs::write(
        &output_schema,
        r#"{"type":"object","required":["governor"],"additionalProperties":false}"#,
    )
    .unwrap();
    fs::write(&dry_run, r#"{"governor":"ondemand"}"#).unwrap();
    fs::write(
        &manual_comparison,
        r#"{"manual_sample":{"governor":"ondemand"},"matches":true}"#,
    )
    .unwrap();
    fs::write(
        &static_safety_review,
        "Read-only cpufreq adapter. No target writes. Output bounded.",
    )
    .unwrap();

    let manifest = workspace_root().join("examples/tools/linux_cpufreq_reader.yaml");
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "tool",
            "qualify",
            "--manifest",
            manifest.to_str().unwrap(),
            "--tool-version",
            "0.1.0",
            "--tool-sha256",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--output-schema",
            output_schema.to_str().unwrap(),
            "--dry-run-output",
            dry_run.to_str().unwrap(),
            "--manual-comparison",
            manual_comparison.to_str().unwrap(),
            "--static-safety-review",
            static_safety_review.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"qualified\""))
        .stdout(contains("\"evidence_accepted\": true"))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let report = &value["value"];
    assert_eq!(
        report["qualification_scope"],
        "agent_created_bounded_observation_adapter"
    );
    for field in [
        "output_schema_ref",
        "dry_run_ref",
        "manual_comparison_ref",
        "static_safety_review_ref",
    ] {
        assert!(report[field]
            .as_str()
            .unwrap()
            .starts_with("artifact://lab/runs/"));
    }
    let report_text = serde_json::to_string(report).unwrap();
    assert!(!report_text.contains(evidence_dir.to_str().unwrap()));
    assert!(temp
        .path()
        .join("tools/linux_cpufreq_reader.output_schema.json")
        .exists());
    assert!(temp
        .path()
        .join("tools/linux_cpufreq_reader.dry_run.json")
        .exists());
    assert!(temp
        .path()
        .join("tools/linux_cpufreq_reader.manual_comparison.json")
        .exists());
    assert!(temp
        .path()
        .join("tools/linux_cpufreq_reader.static_safety_review.txt")
        .exists());

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"tool.qualify\""));
    assert!(audit.contains("\"result\":\"qualified\""));
}

#[test]
fn tool_qualification_rejects_malformed_dry_run_evidence_before_acceptance() {
    let temp = tempfile::tempdir().unwrap();
    let evidence_dir = temp.path().join("evidence-inputs");
    fs::create_dir_all(&evidence_dir).unwrap();
    let output_schema = evidence_dir.join("output-schema.json");
    let dry_run = evidence_dir.join("dry-run.json");
    let manual_comparison = evidence_dir.join("manual-comparison.json");
    let static_safety_review = evidence_dir.join("static-safety-review.txt");
    fs::write(&output_schema, r#"{"type":"object"}"#).unwrap();
    fs::write(&dry_run, "{bad").unwrap();
    fs::write(&manual_comparison, r#"{"matches":true}"#).unwrap();
    fs::write(&static_safety_review, "Read-only adapter review.").unwrap();

    let manifest = workspace_root().join("examples/tools/linux_cpufreq_reader.yaml");
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "tool",
            "qualify",
            "--manifest",
            manifest.to_str().unwrap(),
            "--tool-version",
            "0.1.0",
            "--tool-sha256",
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--output-schema",
            output_schema.to_str().unwrap(),
            "--dry-run-output",
            dry_run.to_str().unwrap(),
            "--manual-comparison",
            manual_comparison.to_str().unwrap(),
            "--static-safety-review",
            static_safety_review.to_str().unwrap(),
            "--run-dir",
            temp.path().to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(contains("dry-run output evidence must be valid JSON"));
    assert!(!temp
        .path()
        .join("tools/linux_cpufreq_reader.qualification.json")
        .exists());
}
