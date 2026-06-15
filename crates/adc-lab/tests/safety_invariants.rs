use adc_lab_core::{
    canonical_plan_digest, Actor, ActorKind, ApprovalBounds, ApprovalRecord, ControlPlan, RiskTier,
};
use assert_cmd::Command;
use predicates::str::contains;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
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
fn contract_validation_control_plan_and_helper_dry_run_refusal_are_structured() {
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
fn contract_validation_control_apply_refuses_remote_plan_without_invoking_helper() {
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
fn contract_validation_control_apply_has_no_public_helper_override() {
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
fn contract_validation_governor_sweep_cannot_self_approve_real_apply() {
    let temp = tempfile::tempdir().unwrap();
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
            "--approved-by",
            "operator",
            "--run-dir",
            temp.path().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("approved sweep policy"));

    let plan_files = fs::read_dir(temp.path().join("plans"))
        .unwrap()
        .flatten()
        .count();
    assert_eq!(plan_files, 0);
}

#[test]
fn contract_validation_restore_has_no_public_helper_override() {
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
fn contract_validation_ssh_runner_rejects_shell_fragment_env() {
    Command::cargo_bin("adc-lab")
        .unwrap()
        .env("ADC_LAB_TARGET_RUNNER", "sh -c adc-lab-target")
        .args(["inventory", "--target", "ssh://pi4-demo"])
        .assert()
        .failure()
        .stderr(contains("fixed adc-lab-target path"));
}

#[cfg(unix)]
#[test]
fn contract_validation_ssh_runner_missing_on_path_has_actionable_diagnostic() {
    let temp = tempfile::tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    let ssh_path = bin_dir.join("ssh");
    fs::write(
        &ssh_path,
        r#"#!/bin/sh
echo "adc-lab-target: not found" >&2
exit 127
"#,
    )
    .unwrap();
    fs::set_permissions(&ssh_path, fs::Permissions::from_mode(0o755)).unwrap();
    let old_path = std::env::var_os("PATH").unwrap_or_default();
    let path_env = format!("{}:{}", bin_dir.display(), old_path.to_string_lossy());
    let run_dir = temp.path().join("run");

    Command::cargo_bin("adc-lab")
        .unwrap()
        .env("PATH", path_env)
        .env_remove("ADC_LAB_TARGET_RUNNER")
        .args([
            "load",
            "cpu",
            "--target",
            "ssh://target55",
            "--workers",
            "1",
            "--duration",
            "1s",
            "--run-dir",
            run_dir.to_str().unwrap(),
            "--json",
        ])
        .assert()
        .failure()
        .stderr(contains("ssh target runner version failed"))
        .stderr(contains("tried_runner=adc-lab-target"))
        .stderr(contains("default_runner=adc-lab-target"))
        .stderr(contains("ADC_LAB_TARGET_RUNNER"))
        .stderr(contains("~/.local/bin/adc-lab-target"))
        .stderr(contains("non-interactive SSH PATH"));
}

#[cfg(unix)]
struct FakeSshHarness {
    temp: tempfile::TempDir,
    bin_dir: PathBuf,
    log_path: PathBuf,
    marker_path: PathBuf,
}

#[cfg(unix)]
impl FakeSshHarness {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let bin_dir = temp.path().join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let log_path = temp.path().join("fake-ssh.log");
        let marker_path = temp.path().join("operator-abort-injected");
        let ssh_path = bin_dir.join("ssh");
        let target_path = bin_dir.join("adc-lab-target");

        fs::write(
            &ssh_path,
            r#"#!/bin/sh
set -eu
bin_dir=$(dirname "$0")
endpoint=$1
shift
{
  printf 'endpoint=%s\n' "$endpoint"
  printf 'remote_command=%s\n' "$*"
} >> "$ADC_LAB_FAKE_SSH_LOG"
PATH="$bin_dir:$PATH"
export PATH
/bin/sh -c "$*"
"#,
        )
        .unwrap();
        fs::write(
            &target_path,
            r#"#!/bin/sh
set -eu
{
  printf 'target_argv:'
  for arg in "$@"; do
    printf ' [%s]' "$arg"
  done
  printf '\n'
} >> "$ADC_LAB_FAKE_SSH_LOG"
if [ "$1" = "--version" ]; then
  cat <<'JSON'
{"name":"adc-lab-target","version":"0.1.0","git_sha":"test","target_triple":"x86_64-unknown-linux-gnu","build_profile":"test"}
JSON
elif [ "$1" = "load" ] && [ "$2" = "cpu" ]; then
  cat <<'JSON'
{"schema_version":"lab.load_result.v1","result_id":"LOAD-RESULT-FAKE","load_id":"LOAD-FAKE","target_id":"fake-target","status":"completed","workers":1,"duration_ms":1,"abort_reason":null,"max_observed_temp_c":null,"worker_iterations":[1],"safety_monitor":{"sample_interval_ms":100,"samples":1,"thermal_surface_available":false,"operator_abort_observed":false,"restore_on_abort_status":"not_required"},"time_unix_ms":1}
JSON
elif [ "$1" = "observe" ]; then
  cat <<'JSON'
{"schema_version":"lab.observation_result.v1","target_id":"fake-target","duration_ms":1,"signals":["cpu","freq","thermal","memory"],"samples":[]}
JSON
else
  echo "unexpected adc-lab-target command" >&2
  exit 2
fi
"#,
        )
        .unwrap();
        fs::set_permissions(&ssh_path, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&target_path, fs::Permissions::from_mode(0o755)).unwrap();

        Self {
            temp,
            bin_dir,
            log_path,
            marker_path,
        }
    }

    fn path_env(&self) -> String {
        let old_path = std::env::var_os("PATH").unwrap_or_default();
        format!("{}:{}", self.bin_dir.display(), old_path.to_string_lossy())
    }

    fn run_dir(&self) -> &std::path::Path {
        self.temp.path()
    }

    fn semicolon_abort_path(&self) -> String {
        format!("x; /usr/bin/id > {}; #", self.marker_path.display())
    }

    fn assert_abort_path_was_argv_data(&self, abort_path: &str) {
        assert!(
            !self.marker_path.exists(),
            "operator abort path was interpreted as remote shell syntax"
        );
        let log = fs::read_to_string(&self.log_path).unwrap();
        assert!(log.contains("remote_command='adc-lab-target' 'load' 'cpu'"));
        assert!(log.contains(&format!("[{}]", abort_path)));
    }
}

#[cfg(unix)]
#[test]
fn contract_validation_load_cpu_ssh_operator_abort_file_is_remote_shell_quoted() {
    let harness = FakeSshHarness::new();
    let abort_path = harness.semicolon_abort_path();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .env("PATH", harness.path_env())
        .env("ADC_LAB_FAKE_SSH_LOG", &harness.log_path)
        .args([
            "load",
            "cpu",
            "--target",
            "ssh://pi4-demo",
            "--workers",
            "1",
            "--duration",
            "1s",
            "--operator-abort-file",
            &abort_path,
            "--run-dir",
            harness.run_dir().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("LOAD-RESULT-FAKE"));

    harness.assert_abort_path_was_argv_data(&abort_path);
}

#[cfg(unix)]
#[test]
fn contract_validation_experiment_ssh_operator_abort_file_is_remote_shell_quoted() {
    let harness = FakeSshHarness::new();
    let matrix = workspace_root().join("examples/experiments/bounded_load_observe_smoke.yaml");
    let abort_path = harness.semicolon_abort_path();

    Command::cargo_bin("adc-lab")
        .unwrap()
        .env("PATH", harness.path_env())
        .env("ADC_LAB_FAKE_SSH_LOG", &harness.log_path)
        .args([
            "experiment",
            "run",
            "--target",
            "ssh://pi4-demo",
            "--matrix",
            matrix.to_str().unwrap(),
            "--trial-load-duration",
            "1s",
            "--trial-observe-duration",
            "0s",
            "--operator-abort-file",
            &abort_path,
            "--run-dir",
            harness.run_dir().to_str().unwrap(),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("experiment_run.json"));

    harness.assert_abort_path_was_argv_data(&abort_path);
}
