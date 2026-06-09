use crate::contracts::{
    Actor, AppliedState, ApprovalRecord, CapturedState, ControlOperation, ControlPlan,
    ControlResult, ControlResultStatus, CpufreqDesiredState, CpufreqPolicyState, OperationBounds,
    Refusal, RefusalCode, RestoreAttempt, RestoreAttemptStatus, RestoreLease, RestoreStatus,
    RiskTier,
};
use crate::error::{IoPathExt, LabResult};
use crate::ids::{new_id, now_unix_ms};
use crate::{LabError, RunContext, TargetSpec, LOCAL_TARGET_ID};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

pub const CPUFREQ_SET_GOVERNOR: &str = "linux.cpufreq.set_governor";
pub const DEFAULT_PRIV_HELPER: &str = "/usr/local/libexec/adc-lab-priv-helper";
pub const ALLOWED_GOVERNORS: &[&str] = &[
    "performance",
    "ondemand",
    "powersave",
    "schedutil",
    "userspace",
];

pub fn new_cpufreq_plan(
    run: &RunContext,
    target: &TargetSpec,
    governor: String,
    duration_seconds_max: u64,
    thermal_celsius_abort: Option<f64>,
) -> ControlPlan {
    ControlPlan {
        schema_version: "lab.control_plan.v1".to_string(),
        plan_id: new_id("PLAN"),
        run_id: run.run_id.clone(),
        target_id: target.target_id.clone(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        approval_required: true,
        restore_required: true,
        operation: ControlOperation {
            operation_id: CPUFREQ_SET_GOVERNOR.to_string(),
            desired_state: CpufreqDesiredState { governor },
        },
        bounds: OperationBounds {
            duration_seconds_max,
            thermal_celsius_abort,
        },
        created_by: Actor::codex(),
        time_unix_ms: now_unix_ms(),
    }
}

pub fn validate_control_plan(plan: &ControlPlan) -> Result<(), Refusal> {
    if plan.operation.operation_id != CPUFREQ_SET_GOVERNOR {
        return Err(refusal(
            RefusalCode::UnsupportedOperation,
            "operation is not allowlisted",
        ));
    }
    if !ALLOWED_GOVERNORS.contains(&plan.operation.desired_state.governor.as_str()) {
        return Err(refusal(
            RefusalCode::PolicyViolation,
            "governor is not in the allowlist",
        ));
    }
    if plan.risk_tier != RiskTier::Tier2PrivilegedReversible
        || !plan.approval_required
        || !plan.restore_required
    {
        return Err(refusal(
            RefusalCode::PolicyViolation,
            "cpufreq governor control must be tier2 with approval and restore",
        ));
    }
    Ok(())
}

pub fn approval_matches(plan: &ControlPlan, approval: &ApprovalRecord) -> Result<(), Refusal> {
    let plan_digest = canonical_plan_digest(plan).map_err(|err| {
        refusal(
            RefusalCode::InvalidPlan,
            format!("failed to calculate plan digest: {err}"),
        )
    })?;
    if approval.target_id != plan.target_id
        || approval.risk_tier != plan.risk_tier
        || !approval.restore_required
        || approval.approved_plan_id != plan.plan_id
        || approval.approved_plan_digest != plan_digest
        || approval.approved_operation != plan.operation
        || !approval
            .approved_actions
            .iter()
            .any(|action| action == &plan.operation.operation_id)
    {
        return Err(refusal(
            RefusalCode::ApprovalMismatch,
            "approval does not match plan id, digest, target, risk, restore, operation, or action",
        ));
    }
    if approval.bounds.duration_seconds_max < plan.bounds.duration_seconds_max {
        return Err(refusal(
            RefusalCode::ApprovalMismatch,
            "approval duration bound is narrower than plan",
        ));
    }
    match (
        approval.bounds.thermal_celsius_abort,
        plan.bounds.thermal_celsius_abort,
    ) {
        (Some(approved), Some(planned)) if planned <= approved => {}
        (None, None) => {}
        _ => {
            return Err(refusal(
                RefusalCode::ApprovalMismatch,
                "approval thermal abort bound does not cover plan",
            ))
        }
    }
    Ok(())
}

pub fn canonical_plan_digest(plan: &ControlPlan) -> LabResult<String> {
    let bytes = serde_json::to_vec(plan)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

pub fn validate_priv_helper_path(path: &Path) -> LabResult<()> {
    let value = path.to_str().ok_or_else(|| {
        LabError::Policy("privileged helper path must be valid UTF-8".to_string())
    })?;
    let allowed = [DEFAULT_PRIV_HELPER, "/usr/bin/adc-lab-priv-helper"];
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(LabError::Policy(format!(
            "privileged helper path is outside the adc-lab allowlist: {value}"
        )))
    }
}

pub fn target_local_helper_refusal(target_id: &str) -> Refusal {
    refusal(
        RefusalCode::PrivilegedApplyRequiresTargetLocalHelper,
        format!(
            "privileged apply/restore is local-target only in this MVP; refused target_id={target_id}"
        ),
    )
}

pub fn refused_result(plan: &ControlPlan, refusal: Refusal) -> ControlResult {
    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: plan.plan_id.clone(),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        risk_tier: plan.risk_tier.clone(),
        status: ControlResultStatus::Refused,
        refusal: Some(refusal),
        restore_lease: None,
        restore_attempted: false,
        restore_result: None,
        time_unix_ms: now_unix_ms(),
    }
}

pub fn dry_run_ok_result(plan: &ControlPlan) -> ControlResult {
    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: plan.plan_id.clone(),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        risk_tier: plan.risk_tier.clone(),
        status: ControlResultStatus::DryRunOk,
        refusal: None,
        restore_lease: None,
        restore_attempted: false,
        restore_result: None,
        time_unix_ms: now_unix_ms(),
    }
}

pub trait CpufreqBackend {
    fn capture(&self) -> LabResult<Vec<CpufreqPolicyState>>;
    fn apply_governor(&self, governor: &str) -> LabResult<()>;
    fn verify_governor(&self, governor: &str) -> LabResult<bool>;
    fn restore(&self, states: &[CpufreqPolicyState]) -> LabResult<()>;
}

#[derive(Debug, Clone)]
pub struct LinuxCpufreqBackend {
    base: PathBuf,
}

impl Default for LinuxCpufreqBackend {
    fn default() -> Self {
        Self::new("/sys/devices/system/cpu/cpufreq")
    }
}

impl LinuxCpufreqBackend {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self { base: base.into() }
    }

    fn policy_dirs(&self) -> LabResult<Vec<PathBuf>> {
        let entries = fs::read_dir(&self.base).with_path(&self.base)?;
        let mut dirs = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with("policy"))
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        dirs.sort();
        if dirs.is_empty() {
            return Err(LabError::MissingSurface(
                "cpufreq policy directory".to_string(),
            ));
        }
        Ok(dirs)
    }
}

impl CpufreqBackend for LinuxCpufreqBackend {
    fn capture(&self) -> LabResult<Vec<CpufreqPolicyState>> {
        self.policy_dirs()?
            .into_iter()
            .map(|policy_dir| {
                let policy = policy_dir
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("policy")
                    .to_string();
                let governor = read_required(policy_dir.join("scaling_governor"))?;
                let scaling_min_freq = read_optional(policy_dir.join("scaling_min_freq"))?;
                let scaling_max_freq = read_optional(policy_dir.join("scaling_max_freq"))?;
                Ok(CpufreqPolicyState {
                    policy,
                    governor,
                    scaling_min_freq,
                    scaling_max_freq,
                })
            })
            .collect()
    }

    fn apply_governor(&self, governor: &str) -> LabResult<()> {
        for policy_dir in self.policy_dirs()? {
            let path = policy_dir.join("scaling_governor");
            fs::write(&path, governor).with_path(path)?;
        }
        Ok(())
    }

    fn verify_governor(&self, governor: &str) -> LabResult<bool> {
        Ok(self
            .capture()?
            .iter()
            .all(|policy| policy.governor == governor))
    }

    fn restore(&self, states: &[CpufreqPolicyState]) -> LabResult<()> {
        for state in states {
            let policy_dir = self.base.join(&state.policy);
            fs::write(policy_dir.join("scaling_governor"), &state.governor)
                .with_path(policy_dir.join("scaling_governor"))?;
        }
        Ok(())
    }
}

pub fn apply_control_plan<B: CpufreqBackend>(
    plan: &ControlPlan,
    approval: Option<&ApprovalRecord>,
    backend: &B,
    dry_run: bool,
) -> ControlResult {
    if let Err(refusal) = validate_control_plan(plan) {
        return refused_result(plan, refusal);
    }
    if plan.target_id != LOCAL_TARGET_ID {
        return refused_result(plan, target_local_helper_refusal(&plan.target_id));
    }
    if plan.approval_required {
        let Some(approval) = approval else {
            return refused_result(
                plan,
                refusal(
                    RefusalCode::ApprovalRequired,
                    "tier2 operation requires approval artifact",
                ),
            );
        };
        if let Err(refusal) = approval_matches(plan, approval) {
            return refused_result(plan, refusal);
        }
    }
    if dry_run {
        return dry_run_ok_result(plan);
    }

    let captured = match backend.capture() {
        Ok(states) => states,
        Err(err) => {
            return refused_result(
                plan,
                refusal(
                    RefusalCode::MissingSurface,
                    format!("failed to capture pre-state: {err}"),
                ),
            )
        }
    };
    if let Err(err) = backend.apply_governor(&plan.operation.desired_state.governor) {
        return failed_result_with_restore_attempt(
            plan,
            refusal(
                RefusalCode::PolicyViolation,
                format!("failed to apply governor: {err}"),
            ),
            captured,
            backend,
        );
    }
    match backend.verify_governor(&plan.operation.desired_state.governor) {
        Ok(true) => {}
        Ok(false) => {
            return failed_result_with_restore_attempt(
                plan,
                refusal(RefusalCode::PolicyViolation, "applied state did not verify"),
                captured,
                backend,
            )
        }
        Err(err) => {
            return failed_result_with_restore_attempt(
                plan,
                refusal(
                    RefusalCode::PolicyViolation,
                    format!("failed to verify applied state: {err}"),
                ),
                captured,
                backend,
            )
        }
    }

    let lease = RestoreLease {
        schema_version: "lab.restore_lease.v1".to_string(),
        lease_id: new_id("LEASE"),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        captured_state: CapturedState {
            cpufreq_policies: captured,
        },
        applied_state: AppliedState {
            governor: plan.operation.desired_state.governor.clone(),
        },
        restore_required: true,
        restore_status: RestoreStatus::Pending,
        created_by_plan: plan.plan_id.clone(),
        time_unix_ms: now_unix_ms(),
    };

    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: plan.plan_id.clone(),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        risk_tier: plan.risk_tier.clone(),
        status: ControlResultStatus::Applied,
        refusal: None,
        restore_lease: Some(lease),
        restore_attempted: false,
        restore_result: None,
        time_unix_ms: now_unix_ms(),
    }
}

pub fn restore_lease<B: CpufreqBackend>(
    lease: &RestoreLease,
    backend: &B,
    dry_run: bool,
) -> ControlResult {
    if let Err(refusal) = validate_restore_lease(lease) {
        return restore_refused_result(lease, refusal);
    }
    if lease.target_id != LOCAL_TARGET_ID {
        return restore_refused_result(lease, target_local_helper_refusal(&lease.target_id));
    }
    if dry_run {
        return ControlResult {
            schema_version: "lab.control_result.v1".to_string(),
            result_id: new_id("RESULT"),
            plan_id: lease.created_by_plan.clone(),
            target_id: lease.target_id.clone(),
            operation_id: lease.operation_id.clone(),
            risk_tier: RiskTier::Tier2PrivilegedReversible,
            status: ControlResultStatus::DryRunOk,
            refusal: None,
            restore_lease: Some(RestoreLease {
                restore_status: RestoreStatus::DryRun,
                ..lease.clone()
            }),
            restore_attempted: false,
            restore_result: None,
            time_unix_ms: now_unix_ms(),
        };
    }

    match backend.restore(&lease.captured_state.cpufreq_policies) {
        Ok(()) => match verify_restored_state(lease, backend) {
            Ok(()) => ControlResult {
                schema_version: "lab.control_result.v1".to_string(),
                result_id: new_id("RESULT"),
                plan_id: lease.created_by_plan.clone(),
                target_id: lease.target_id.clone(),
                operation_id: lease.operation_id.clone(),
                risk_tier: RiskTier::Tier2PrivilegedReversible,
                status: ControlResultStatus::Restored,
                refusal: None,
                restore_lease: Some(RestoreLease {
                    restore_status: RestoreStatus::Restored,
                    ..lease.clone()
                }),
                restore_attempted: true,
                restore_result: Some(RestoreAttempt {
                    status: RestoreAttemptStatus::Succeeded,
                    message: "restore command completed and verified".to_string(),
                }),
                time_unix_ms: now_unix_ms(),
            },
            Err(err) => restore_failed_result(lease, format!("restore verification failed: {err}")),
        },
        Err(err) => ControlResult {
            schema_version: "lab.control_result.v1".to_string(),
            result_id: new_id("RESULT"),
            plan_id: lease.created_by_plan.clone(),
            target_id: lease.target_id.clone(),
            operation_id: lease.operation_id.clone(),
            risk_tier: RiskTier::Tier2PrivilegedReversible,
            status: ControlResultStatus::Failed,
            refusal: Some(refusal(
                RefusalCode::PolicyViolation,
                format!("restore failed: {err}"),
            )),
            restore_lease: Some(RestoreLease {
                restore_status: RestoreStatus::Failed,
                ..lease.clone()
            }),
            restore_attempted: true,
            restore_result: Some(RestoreAttempt {
                status: RestoreAttemptStatus::Failed,
                message: format!("restore failed: {err}"),
            }),
            time_unix_ms: now_unix_ms(),
        },
    }
}

fn validate_restore_lease(lease: &RestoreLease) -> Result<(), Refusal> {
    if lease.schema_version != "lab.restore_lease.v1" {
        return Err(refusal(
            RefusalCode::InvalidPlan,
            "restore lease schema_version must be lab.restore_lease.v1",
        ));
    }
    if lease.operation_id != CPUFREQ_SET_GOVERNOR {
        return Err(refusal(
            RefusalCode::UnsupportedOperation,
            "restore lease operation is not allowlisted",
        ));
    }
    if !lease.restore_required {
        return Err(refusal(
            RefusalCode::InvalidPlan,
            "restore lease must require restore",
        ));
    }
    if !ALLOWED_GOVERNORS.contains(&lease.applied_state.governor.as_str()) {
        return Err(refusal(
            RefusalCode::PolicyViolation,
            "restore lease applied governor is not in the allowlist",
        ));
    }
    if lease.captured_state.cpufreq_policies.is_empty() {
        return Err(refusal(
            RefusalCode::MissingSurface,
            "restore lease has no captured cpufreq policy state",
        ));
    }
    for state in &lease.captured_state.cpufreq_policies {
        validate_policy_segment(&state.policy)?;
        validate_governor_value(&state.governor)?;
        validate_optional_frequency("scaling_min_freq", state.scaling_min_freq.as_deref())?;
        validate_optional_frequency("scaling_max_freq", state.scaling_max_freq.as_deref())?;
    }
    Ok(())
}

fn validate_policy_segment(policy: &str) -> Result<(), Refusal> {
    let suffix = policy.strip_prefix("policy").ok_or_else(|| {
        refusal(
            RefusalCode::PolicyViolation,
            "restore lease policy must match policy[0-9]+",
        )
    })?;
    if suffix.is_empty() || !suffix.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(refusal(
            RefusalCode::PolicyViolation,
            "restore lease policy must match policy[0-9]+",
        ));
    }
    Ok(())
}

fn validate_governor_value(governor: &str) -> Result<(), Refusal> {
    if ALLOWED_GOVERNORS.contains(&governor) {
        Ok(())
    } else {
        Err(refusal(
            RefusalCode::PolicyViolation,
            "restore lease governor is not in the allowlist",
        ))
    }
}

fn validate_optional_frequency(label: &str, value: Option<&str>) -> Result<(), Refusal> {
    if let Some(value) = value {
        if value.is_empty() || !value.chars().all(|ch| ch.is_ascii_digit()) {
            return Err(refusal(
                RefusalCode::PolicyViolation,
                format!("restore lease {label} must be numeric"),
            ));
        }
    }
    Ok(())
}

fn verify_restored_state<B: CpufreqBackend>(lease: &RestoreLease, backend: &B) -> LabResult<()> {
    let current = backend.capture()?;
    for expected in &lease.captured_state.cpufreq_policies {
        let Some(actual) = current.iter().find(|state| state.policy == expected.policy) else {
            return Err(LabError::Policy(format!(
                "restored policy {} was not found during verification",
                expected.policy
            )));
        };
        if actual.governor != expected.governor {
            return Err(LabError::Policy(format!(
                "restored policy {} governor mismatch: expected {}, got {}",
                expected.policy, expected.governor, actual.governor
            )));
        }
    }
    Ok(())
}

fn restore_failed_result(lease: &RestoreLease, message: String) -> ControlResult {
    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: lease.created_by_plan.clone(),
        target_id: lease.target_id.clone(),
        operation_id: lease.operation_id.clone(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        status: ControlResultStatus::Failed,
        refusal: Some(refusal(RefusalCode::PolicyViolation, message.clone())),
        restore_lease: Some(RestoreLease {
            restore_status: RestoreStatus::Failed,
            ..lease.clone()
        }),
        restore_attempted: true,
        restore_result: Some(RestoreAttempt {
            status: RestoreAttemptStatus::Failed,
            message,
        }),
        time_unix_ms: now_unix_ms(),
    }
}

pub fn restore_refused_result(lease: &RestoreLease, refusal: Refusal) -> ControlResult {
    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: lease.created_by_plan.clone(),
        target_id: lease.target_id.clone(),
        operation_id: lease.operation_id.clone(),
        risk_tier: RiskTier::Tier2PrivilegedReversible,
        status: ControlResultStatus::Refused,
        refusal: Some(refusal),
        restore_lease: Some(lease.clone()),
        restore_attempted: false,
        restore_result: None,
        time_unix_ms: now_unix_ms(),
    }
}

fn failed_result_with_restore_attempt<B: CpufreqBackend>(
    plan: &ControlPlan,
    refusal: Refusal,
    captured: Vec<CpufreqPolicyState>,
    backend: &B,
) -> ControlResult {
    let restore_outcome = backend.restore(&captured);
    let (restore_status, restore_result) = match restore_outcome {
        Ok(()) => (
            RestoreStatus::Restored,
            RestoreAttempt {
                status: RestoreAttemptStatus::Succeeded,
                message: "restore attempted after apply failure and succeeded".to_string(),
            },
        ),
        Err(err) => (
            RestoreStatus::Failed,
            RestoreAttempt {
                status: RestoreAttemptStatus::Failed,
                message: format!("restore attempted after apply failure and failed: {err}"),
            },
        ),
    };
    let lease = RestoreLease {
        schema_version: "lab.restore_lease.v1".to_string(),
        lease_id: new_id("LEASE"),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        captured_state: CapturedState {
            cpufreq_policies: captured,
        },
        applied_state: AppliedState {
            governor: plan.operation.desired_state.governor.clone(),
        },
        restore_required: true,
        restore_status,
        created_by_plan: plan.plan_id.clone(),
        time_unix_ms: now_unix_ms(),
    };

    ControlResult {
        schema_version: "lab.control_result.v1".to_string(),
        result_id: new_id("RESULT"),
        plan_id: plan.plan_id.clone(),
        target_id: plan.target_id.clone(),
        operation_id: plan.operation.operation_id.clone(),
        risk_tier: plan.risk_tier.clone(),
        status: ControlResultStatus::Failed,
        refusal: Some(refusal),
        restore_lease: Some(lease),
        restore_attempted: true,
        restore_result: Some(restore_result),
        time_unix_ms: now_unix_ms(),
    }
}

fn refusal(reason_code: RefusalCode, message: impl Into<String>) -> Refusal {
    Refusal {
        reason_code,
        message: message.into(),
    }
}

fn read_required(path: impl AsRef<Path>) -> LabResult<String> {
    let path = path.as_ref();
    Ok(fs::read_to_string(path).with_path(path)?.trim().to_string())
}

fn read_optional(path: impl AsRef<Path>) -> LabResult<Option<String>> {
    let path = path.as_ref();
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value.trim().to_string())),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(crate::LabError::IoWithPath {
            path: path.to_path_buf(),
            source: err,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ActorKind, ApprovalBounds};

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
}
