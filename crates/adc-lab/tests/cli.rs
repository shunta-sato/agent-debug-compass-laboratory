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
    let output = Command::cargo_bin("adc-lab")
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
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let plan_path = value["artifact_path"].as_str().unwrap();

    Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "adc-lab-priv-helper",
            "--",
            "apply",
            "--plan",
            plan_path,
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
    let output = Command::cargo_bin("adc-lab")
        .unwrap()
        .args([
            "control",
            "plan",
            "--target",
            "ssh://target55",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "cpu.governor",
            "--set",
            "performance",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let plan_path = value["artifact_path"].as_str().unwrap();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .args(["control", "apply", "--plan", plan_path])
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
    let output = Command::cargo_bin("adc-lab")
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
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let plan_path = value["artifact_path"].as_str().unwrap();
    let plan: ControlPlan = serde_json::from_slice(&fs::read(plan_path).unwrap()).unwrap();
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
            plan_path,
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
    let output = Command::cargo_bin("adc-lab")
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
        .get_output()
        .stdout
        .clone();
    let value: serde_json::Value = serde_json::from_slice(&output).unwrap();
    let plan_path = value["artifact_path"].as_str().unwrap();
    let plan: ControlPlan = serde_json::from_slice(&fs::read(plan_path).unwrap()).unwrap();
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
            plan_path,
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
        .args(["inventory", "--target", "ssh://target55"])
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
