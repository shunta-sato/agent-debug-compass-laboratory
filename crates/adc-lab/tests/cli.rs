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
