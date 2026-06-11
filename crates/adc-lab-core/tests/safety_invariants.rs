use adc_lab_core::{
    apply_control_plan, canonical_plan_digest, restore_lease, ssh_runner_program,
    validate_priv_helper_path, validate_ssh_runner_program, Actor, ActorKind, AppliedState,
    ApprovalBounds, ApprovalRecord, CapturedState, ControlOperation, ControlPlan,
    ControlResultStatus, CpufreqBackend, CpufreqDesiredState, CpufreqPolicyState, OperationBounds,
    RefusalCode, RestoreAttemptStatus, RestoreLease, RestoreStatus, RiskTier, TargetSpec,
    TargetTransport, CPUFREQ_SET_GOVERNOR, DEFAULT_PRIV_HELPER, LOCAL_TARGET_ID,
};
use adc_lab_core::{LabError, LabResult};
use std::path::Path;

#[derive(Clone)]
struct FakeBackend {
    states: Vec<CpufreqPolicyState>,
}

impl CpufreqBackend for FakeBackend {
    fn capture(&self) -> LabResult<Vec<CpufreqPolicyState>> {
        Ok(self.states.clone())
    }

    fn apply_governor(&self, _governor: &str) -> LabResult<()> {
        Ok(())
    }

    fn verify_governor(&self, _governor: &str) -> LabResult<bool> {
        Ok(true)
    }

    fn restore(&self, _states: &[CpufreqPolicyState]) -> LabResult<()> {
        Ok(())
    }
}

struct FailingApplyBackend {
    states: Vec<CpufreqPolicyState>,
}

impl CpufreqBackend for FailingApplyBackend {
    fn capture(&self) -> LabResult<Vec<CpufreqPolicyState>> {
        Ok(self.states.clone())
    }

    fn apply_governor(&self, _governor: &str) -> LabResult<()> {
        Err(LabError::Policy("simulated write failure".to_string()))
    }

    fn verify_governor(&self, _governor: &str) -> LabResult<bool> {
        Ok(false)
    }

    fn restore(&self, states: &[CpufreqPolicyState]) -> LabResult<()> {
        assert_eq!(states.len(), 1);
        Ok(())
    }
}

struct RestoreMismatchBackend {
    states: Vec<CpufreqPolicyState>,
}

impl CpufreqBackend for RestoreMismatchBackend {
    fn capture(&self) -> LabResult<Vec<CpufreqPolicyState>> {
        Ok(self.states.clone())
    }

    fn apply_governor(&self, _governor: &str) -> LabResult<()> {
        Ok(())
    }

    fn verify_governor(&self, _governor: &str) -> LabResult<bool> {
        Ok(true)
    }

    fn restore(&self, _states: &[CpufreqPolicyState]) -> LabResult<()> {
        Ok(())
    }
}

fn plan() -> ControlPlan {
    ControlPlan {
        schema_version: "lab.control_plan.v1".to_string(),
        plan_id: "PLAN-001".to_string(),
        run_id: "LAB-RUN-001".to_string(),
        target_id: LOCAL_TARGET_ID.to_string(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        approval_required: true,
        restore_required: true,
        operation: ControlOperation {
            operation_id: CPUFREQ_SET_GOVERNOR.to_string(),
            desired_state: CpufreqDesiredState {
                governor: "performance".to_string(),
            },
        },
        bounds: OperationBounds {
            duration_seconds_max: 60,
            thermal_celsius_abort: Some(75.0),
        },
        created_by: Actor::codex(),
        time_unix_ms: 1,
    }
}

fn lease() -> RestoreLease {
    RestoreLease {
        schema_version: "lab.restore_lease.v1".to_string(),
        lease_id: "LEASE-001".to_string(),
        target_id: LOCAL_TARGET_ID.to_string(),
        operation_id: CPUFREQ_SET_GOVERNOR.to_string(),
        captured_state: CapturedState {
            cpufreq_policies: vec![CpufreqPolicyState {
                policy: "policy0".to_string(),
                governor: "ondemand".to_string(),
                scaling_min_freq: Some("600000".to_string()),
                scaling_max_freq: Some("1800000".to_string()),
            }],
        },
        applied_state: AppliedState {
            governor: "performance".to_string(),
        },
        restore_required: true,
        restore_status: RestoreStatus::Pending,
        created_by_plan: "PLAN-001".to_string(),
        time_unix_ms: 1,
    }
}

fn approval() -> ApprovalRecord {
    let plan = plan();
    ApprovalRecord {
        schema_version: "lab.approval_record.v1".to_string(),
        approval_id: "APPROVAL-001".to_string(),
        target_id: plan.target_id.clone(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        operation_summary: "Set CPU governor".to_string(),
        approved_plan_id: plan.plan_id.clone(),
        approved_plan_digest: canonical_plan_digest(&plan).unwrap(),
        approved_operation: plan.operation.clone(),
        approved_by: Actor {
            kind: ActorKind::Human,
            id: "operator".to_string(),
        },
        bounds: ApprovalBounds {
            duration_seconds_max: 60,
            thermal_celsius_abort: Some(75.0),
        },
        restore_required: true,
        approved_actions: vec![CPUFREQ_SET_GOVERNOR.to_string()],
        time_unix_ms: 1,
    }
}

#[test]
fn contract_validation_apply_requires_approval() {
    let result = apply_control_plan(&plan(), None, &FakeBackend { states: Vec::new() }, false);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::ApprovalRequired
    );
}

#[test]
fn contract_validation_apply_creates_restore_lease() {
    let result = apply_control_plan(
        &plan(),
        Some(&approval()),
        &FakeBackend {
            states: vec![CpufreqPolicyState {
                policy: "policy0".to_string(),
                governor: "ondemand".to_string(),
                scaling_min_freq: Some("600000".to_string()),
                scaling_max_freq: Some("1800000".to_string()),
            }],
        },
        false,
    );
    assert_eq!(result.status, ControlResultStatus::Applied);
    assert!(result.restore_lease.is_some());
}

#[test]
fn contract_validation_apply_rejects_non_local_privileged_plan() {
    let mut remote_plan = plan();
    remote_plan.target_id = "target55".to_string();
    let mut approval = approval();
    approval.target_id = remote_plan.target_id.clone();
    approval.approved_plan_digest = canonical_plan_digest(&remote_plan).unwrap();
    let result = apply_control_plan(
        &remote_plan,
        Some(&approval),
        &FakeBackend { states: Vec::new() },
        false,
    );
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PrivilegedApplyRequiresTargetLocalHelper
    );
}

#[test]
fn contract_validation_approval_is_bound_to_plan_digest() {
    let mut approval = approval();
    approval.approved_plan_digest = "sha256:not-the-plan".to_string();
    let result = apply_control_plan(
        &plan(),
        Some(&approval),
        &FakeBackend { states: Vec::new() },
        true,
    );
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::ApprovalMismatch
    );
}

#[test]
fn contract_validation_apply_failure_attempts_restore() {
    let result = apply_control_plan(
        &plan(),
        Some(&approval()),
        &FailingApplyBackend {
            states: vec![CpufreqPolicyState {
                policy: "policy0".to_string(),
                governor: "ondemand".to_string(),
                scaling_min_freq: None,
                scaling_max_freq: None,
            }],
        },
        false,
    );
    assert_eq!(result.status, ControlResultStatus::Failed);
    assert!(result.restore_attempted);
    assert_eq!(
        result.restore_result.unwrap().status,
        RestoreAttemptStatus::Succeeded
    );
    assert_eq!(
        result.restore_lease.unwrap().restore_status,
        RestoreStatus::Restored
    );
}

#[test]
fn contract_validation_restore_rejects_forged_policy_segment() {
    let mut lease = lease();
    lease.captured_state.cpufreq_policies[0].policy = "../policy0".to_string();
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PolicyViolation
    );
}

#[test]
fn contract_validation_restore_rejects_absolute_policy_segment() {
    let mut lease = lease();
    lease.captured_state.cpufreq_policies[0].policy = "/sys/policy0".to_string();
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PolicyViolation
    );
}

#[test]
fn contract_validation_restore_rejects_forged_governor() {
    let mut lease = lease();
    lease.captured_state.cpufreq_policies[0].governor = "bad value".to_string();
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PolicyViolation
    );
}

#[test]
fn contract_validation_restore_rejects_governor_with_newline() {
    let mut lease = lease();
    lease.captured_state.cpufreq_policies[0].governor = "performance\nbad".to_string();
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PolicyViolation
    );
}

#[test]
fn contract_validation_restore_rejects_wrong_schema_version() {
    let mut lease = lease();
    lease.schema_version = "lab.restore_lease.v0".to_string();
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::InvalidPlan
    );
}

#[test]
fn contract_validation_restore_rejects_non_numeric_frequency() {
    let mut lease = lease();
    lease.captured_state.cpufreq_policies[0].scaling_min_freq = Some("600000 bad".to_string());
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::PolicyViolation
    );
}

#[test]
fn contract_validation_restore_requires_restore_required_true() {
    let mut lease = lease();
    lease.restore_required = false;
    let result = restore_lease(&lease, &FakeBackend { states: Vec::new() }, true);
    assert_eq!(result.status, ControlResultStatus::Refused);
    assert_eq!(
        result.refusal.unwrap().reason_code,
        RefusalCode::InvalidPlan
    );
}

#[test]
fn contract_validation_restore_verifies_read_back_state() {
    let result = restore_lease(
        &lease(),
        &RestoreMismatchBackend {
            states: vec![CpufreqPolicyState {
                policy: "policy0".to_string(),
                governor: "performance".to_string(),
                scaling_min_freq: None,
                scaling_max_freq: None,
            }],
        },
        false,
    );
    assert_eq!(result.status, ControlResultStatus::Failed);
    assert!(result.restore_attempted);
    assert_eq!(
        result.restore_result.unwrap().status,
        RestoreAttemptStatus::Failed
    );
}

#[test]
fn contract_validation_priv_helper_path_is_allowlisted() {
    assert!(validate_priv_helper_path(Path::new(DEFAULT_PRIV_HELPER)).is_ok());
    assert!(validate_priv_helper_path(Path::new("/tmp/adc-lab-priv-helper")).is_err());
    assert!(validate_priv_helper_path(Path::new("adc-lab-priv-helper")).is_err());
}

#[test]
fn contract_validation_unknown_plan_field_is_rejected() {
    let mut value = serde_json::to_value(plan()).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("shell".to_string(), serde_json::json!("sudo sh"));
    let error = serde_json::from_value::<ControlPlan>(value).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn contract_validation_target_parse_ssh() {
    let target = TargetSpec::parse("ssh://pi4").unwrap();
    assert_eq!(target.transport, TargetTransport::Ssh);
    assert_eq!(target.target_id, "pi4");
}

#[test]
fn contract_validation_target_parse_ssh_rejects_option_injection() {
    assert!(TargetSpec::parse("ssh://-oProxyCommand=bad").is_err());
    assert!(TargetSpec::parse("ssh://target55;sh").is_err());
    assert!(TargetSpec::parse("ssh://operator@target55").is_ok());
}

#[test]
fn contract_validation_default_ssh_runner_is_fixed() {
    std::env::remove_var("ADC_LAB_TARGET_RUNNER");
    assert_eq!(ssh_runner_program().unwrap(), "adc-lab-target");
}

#[test]
fn contract_validation_ssh_runner_rejects_shell_fragments() {
    assert!(validate_ssh_runner_program("sh -c adc-lab-target").is_err());
    assert!(validate_ssh_runner_program("/tmp/adc-lab-target").is_err());
    assert!(validate_ssh_runner_program("/home/demo/.local/bin/adc-lab-target").is_ok());
    assert!(validate_ssh_runner_program(
        "/home/demo/.local/share/adc-lab/runners/20260610/adc-lab-target"
    )
    .is_ok());
}
