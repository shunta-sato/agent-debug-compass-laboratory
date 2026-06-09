use adc_lab_core::{
    canonical_plan_digest, Actor, ActorKind, ApprovalBounds, ApprovalRecord, ControlPlan, RiskTier,
};
use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;

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
fn control_plan_and_helper_dry_run_refusal_are_structured() {
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

    Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "adc-lab-priv-helper",
            "--",
            "apply",
            "--plan",
            plan_path.to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success()
        .stdout(contains("\"status\": \"refused\""))
        .stdout(contains("\"reason_code\": \"approval_required\""));
}

#[test]
fn control_apply_refuses_remote_plan_without_invoking_helper() {
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
        .success()
        .stdout(contains("\"artifact_ref\""));
    let plan_path = single_plan_path(temp.path());

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args(["control", "apply", "--plan", plan_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(contains("\"status\": \"refused\""))
        .stdout(contains(
            "\"reason_code\": \"privileged_apply_requires_target_local_helper\"",
        ));
}

#[test]
fn control_apply_has_no_public_helper_override() {
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
            "--helper",
            "/tmp/adc-lab-priv-helper",
        ])
        .assert()
        .failure()
        .stderr(contains("unexpected argument '--helper'"));
}

#[test]
fn restore_has_no_public_helper_override() {
    Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "restore",
            "--lease",
            "/tmp/lease.json",
            "--helper",
            "/tmp/adc-lab-priv-helper",
        ])
        .assert()
        .failure()
        .stderr(contains("unexpected argument '--helper'"));
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
        .stdout(contains("\"status\": \"aborted\""))
        .stdout(contains("\"abort_reason\": \"operator_abort\""));

    let abort_path_text = abort_file.to_str().unwrap();
    let plan_path = single_load_artifact_path(temp.path(), ".plan.json");
    let result_path = single_load_artifact_path(temp.path(), ".result.json");
    let plan_text = fs::read_to_string(&plan_path).unwrap();
    let result_text = fs::read_to_string(&result_path).unwrap();
    assert!(!plan_text.contains(abort_path_text));
    assert!(!result_text.contains(abort_path_text));

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
    assert_eq!(result["status"], "aborted");
    assert_eq!(result["abort_reason"], "operator_abort");
    assert_eq!(
        result["safety_monitor"]["operator_abort_observed"],
        serde_json::json!(true)
    );
    assert_eq!(
        result["safety_monitor"]["restore_on_abort_status"],
        serde_json::json!("not_required")
    );

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"load.cpu\""));
    assert!(audit.contains("\"result\":\"aborted\""));
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
fn ssh_runner_rejects_shell_fragment_env() {
    Command::cargo_bin("adc-lab")
        .unwrap()
        .env("ADC_LAB_TARGET_RUNNER", "sh -c adc-lab-target")
        .args(["inventory", "--target", "ssh://pi4-demo"])
        .assert()
        .failure()
        .stderr(contains("fixed adc-lab-target path"));
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
        .stdout(contains("familiarization_pack.json"))
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let refs = value["value"]["artifact_refs"].as_array().unwrap();
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
    assert!(trials.iter().any(|trial| trial["artifact_refs"]
        .as_array()
        .unwrap()
        .iter()
        .any(|artifact| artifact.as_str().unwrap().ends_with("/load_result.json"))));

    let trace: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("reports/claim_evidence_trace.json")).unwrap(),
    )
    .unwrap();
    assert!(trace["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "supported"
            && claim["claim"]
                .as_str()
                .unwrap()
                .contains("Bounded non-privileged experiment matrix executed")
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

    let trace: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("reports/claim_evidence_trace.json")).unwrap(),
    )
    .unwrap();
    assert!(trace["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "blocked"
            && claim["claim"]
                .as_str()
                .unwrap()
                .contains("Bounded non-privileged experiment matrix")
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
        .stdout(contains("operating_point_coverage.json"))
        .stdout(contains("\"coverage_status\": \"observational_only\""))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let coverage = &value["coverage"];
    assert_eq!(coverage["coverage_status"], "observational_only");
    assert!(coverage["observed_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "default_policy_frequency"
            && point["coverage_status"] == "observational_only"));
    assert!(coverage["blocked_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "fixed_cpu_frequency"
            && point["coverage_status"] == "not_controllable"));
    assert!(coverage["claim_boundaries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|boundary| boundary["decision"] == "blocked"
            && boundary["claim"]
                .as_str()
                .unwrap()
                .contains("fixed CPU frequencies")));
    let cost_model = &value["cost_model"];
    assert_eq!(cost_model["model_status"], "host_fallback_only");
    assert!(cost_model["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|capability| capability["capability_id"] == "cpu_topology"
            && capability["status"] == "observed"));
    assert!(cost_model["architecture_options"]
        .as_array()
        .unwrap()
        .iter()
        .any(|option| option["option_id"] == "gpu_offload" && option["decision"] == "blocked"));
    assert!(cost_model["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim["decision"] == "blocked"
            && claim["claim"].as_str().unwrap().contains("GPU presence")));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.operating_point\""));
    assert!(audit.contains("\"result\":\"observational_only\""));
    assert!(audit.contains("\"operation\":\"report.capability_cost\""));
    assert!(audit.contains("\"result\":\"host_fallback_only\""));
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
    let coverage = &value["coverage"];
    assert_eq!(coverage["coverage_status"], "controlled_subset");
    assert!(coverage["controlled_points"]
        .as_array()
        .unwrap()
        .iter()
        .any(|point| point["factor_id"] == "cpu_load_workers"
            && point["coverage_status"] == "controlled_subset"
            && point["evidence_class"] == "bounded_load"));
    assert!(coverage["claim_boundaries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|boundary| boundary["decision"] == "supported"
            && boundary["claim"]
                .as_str()
                .unwrap()
                .contains("bounded workload")));
    assert!(coverage["claim_boundaries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|boundary| boundary["decision"] == "blocked"
            && boundary["claim"]
                .as_str()
                .unwrap()
                .contains("fixed CPU frequencies")));
    let cost_model = &value["cost_model"];
    assert_eq!(cost_model["model_status"], "host_fallback_only");
    assert!(cost_model["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |capability| capability["capability_id"] == "bounded_cpu_load_response"
                && capability["status"] == "observed"
                && capability["evidence_refs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|artifact| artifact.as_str().unwrap().ends_with("/load_result.json"))
        ));
    assert!(cost_model["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim["decision"] == "blocked"
            && claim["claim"]
                .as_str()
                .unwrap()
                .contains("bounded CPU load proves production readiness")));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.operating_point\""));
    assert!(audit.contains("\"result\":\"controlled_subset\""));
    assert!(audit.contains("\"operation\":\"report.capability_cost\""));
    assert!(audit.contains("\"result\":\"host_fallback_only\""));
}

#[test]
fn report_capability_profile_links_workload_to_run_evidence_without_selection_claim() {
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
            "1s",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success();

    let workload = workspace_root().join("examples/workloads/bounded_cpu_load_2_workers_60s.json");
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "report",
            "capability-profile",
            "--run",
            temp.path().to_str().unwrap(),
            "--target-id",
            "local-target",
            "--workload",
            workload.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains(
            "target_capability_profile.bounded_cpu_load_2_workers_60s.json",
        ))
        .stdout(contains("\"selection_ready\": false"))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let artifact_ref = value["target_capability_profile_ref"].as_str().unwrap();
    assert!(artifact_ref.starts_with("artifact://lab/runs/"));
    assert!(!artifact_ref.contains(temp.path().to_str().unwrap()));
    let profile = &value["profile"];
    assert_eq!(
        profile["schema_version"],
        "lab.target_capability_profile.v1"
    );
    assert_eq!(profile["workload_id"], "bounded_cpu_load_2_workers_60s");
    assert_eq!(profile["selection_ready"], false);
    assert_eq!(profile["observed_results"]["load_result_count"], 1);
    assert!(
        profile["observed_results"]["observation_sample_count"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert!(profile["evidence_refs"]
        .as_array()
        .unwrap()
        .iter()
        .all(|artifact| artifact
            .as_str()
            .unwrap()
            .starts_with("artifact://lab/runs/")));
    let profile_text = serde_json::to_string(profile).unwrap();
    assert!(!profile_text.contains(temp.path().to_str().unwrap()));
    assert!(profile["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim == "Pi4 is sufficient for this workload"));
    assert!(profile["blocked_claims"]
        .as_array()
        .unwrap()
        .iter()
        .any(|claim| claim == "Pi5 is required for this workload"));
    assert!(temp
        .path()
        .join("reports/target_capability_profile.bounded_cpu_load_2_workers_60s.json")
        .exists());

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    assert!(audit.contains("\"operation\":\"report.target_capability_profile\""));
    assert!(audit.contains("\"operation_id\":\"bounded_cpu_load_2_workers_60s\""));
}

#[test]
fn familiarize_read_only_writes_manifest_pack_claim_trace_and_audit() {
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
        .stdout(contains("\"pack_status\": \"observational_read_only\""))
        .stdout(contains("\"run_manifest_ref\""))
        .stdout(contains("\"familiarization_pack_ref\""))
        .stdout(contains("tool qualification summary was generated"))
        .get_output()
        .stdout
        .clone();

    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert!(value["run_manifest_ref"]
        .as_str()
        .unwrap()
        .starts_with("artifact://lab/runs/"));
    assert!(value["familiarization_pack_ref"]
        .as_str()
        .unwrap()
        .starts_with("artifact://lab/runs/"));

    assert!(temp.path().join("run_manifest.json").exists());
    assert!(temp
        .path()
        .join("reports/familiarization_pack.json")
        .exists());
    assert!(temp
        .path()
        .join("reports/claim_evidence_trace.json")
        .exists());
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
    assert!(manifest["data_quality"]["missing"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item == "no controlled operating point experiment was run"));

    let trace: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join("reports/claim_evidence_trace.json")).unwrap(),
    )
    .unwrap();
    assert!(trace["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "supported"
            && claim["claim"]
                .as_str()
                .unwrap()
                .contains("tool qualification summary")
    }));
    assert!(trace["claims"].as_array().unwrap().iter().any(|claim| {
        claim["decision"] == "blocked"
            && claim["claim"]
                .as_str()
                .unwrap()
                .contains("fixed CPU frequency")
    }));

    let audit = fs::read_to_string(temp.path().join("audit.jsonl")).unwrap();
    for operation in [
        "inventory",
        "toolchain.discover",
        "observe",
        "tool.qualify_inventory",
        "report.claim_trace",
        "run_manifest.write",
        "report.pack",
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
