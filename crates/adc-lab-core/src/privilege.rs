use crate::contracts::{
    BuildInfo, PrivilegeDoctorCheck, PrivilegeDoctorCheckStatus, PrivilegeDoctorReport,
    PrivilegeDoctorStatus, PrivilegeProviderAvailability, PrivilegeProviderDescriptor,
    PrivilegeProviderKind, PrivilegeProviderStatus, PrivilegeProviderTransport, PrivilegeSetupPlan,
    PrivilegeSetupPlanKind,
};
use crate::control::{CPUFREQ_SET_GOVERNOR, DEFAULT_PRIV_HELPER};
use crate::ids::{new_id, now_unix_ms};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

pub const OPTION_A_PROVIDER_ID: &str = "option_a_sudo_helper";
pub const OPTION_B_PROVIDER_ID: &str = "option_b_systemd_unix_socket";
pub const OPTION_B_PLANNED_SOCKET: &str = "/run/adc-lab/privileged.sock";

pub fn privilege_provider_status(target_id: String) -> PrivilegeProviderStatus {
    PrivilegeProviderStatus {
        schema_version: "lab.privilege_provider_status.v1".to_string(),
        target_id,
        active_provider_id: OPTION_A_PROVIDER_ID.to_string(),
        providers: vec![option_a_provider(), option_b_provider()],
        time_unix_ms: now_unix_ms(),
    }
}

pub fn privilege_doctor(target_id: String, local_target: bool) -> PrivilegeDoctorReport {
    let helper_path = DEFAULT_PRIV_HELPER.to_string();
    if !local_target {
        return PrivilegeDoctorReport {
            schema_version: "lab.privilege_doctor.v1".to_string(),
            target_id,
            helper_path,
            helper_installed: false,
            root_owned: None,
            world_writable: None,
            sudo_non_interactive_available: false,
            helper_version: None,
            status: PrivilegeDoctorStatus::UnsupportedTarget,
            checks: vec![check(
                "target.local_only",
                PrivilegeDoctorCheckStatus::NotApplicable,
                "privilege doctor is local-target only in this slice",
                Vec::new(),
            )],
            next_action: "run privilege doctor on the target host after operator setup".to_string(),
            time_unix_ms: now_unix_ms(),
        };
    }

    let helper = PathBuf::from(DEFAULT_PRIV_HELPER);
    let metadata = fs::metadata(&helper).ok();
    let helper_installed = metadata.is_some();
    let root_owned = metadata.as_ref().and_then(root_owned);
    let world_writable = metadata.as_ref().and_then(world_writable);
    let helper_version = helper_version(&helper);
    let sudo_non_interactive_available = sudo_non_interactive_version(&helper).is_some();

    let mut checks = Vec::new();
    checks.push(check(
        "helper.exists",
        if helper_installed {
            PrivilegeDoctorCheckStatus::Pass
        } else {
            PrivilegeDoctorCheckStatus::Fail
        },
        if helper_installed {
            "helper path exists"
        } else {
            "helper path is missing"
        },
        vec![DEFAULT_PRIV_HELPER.to_string()],
    ));
    checks.push(check(
        "helper.root_owned",
        match root_owned {
            Some(true) => PrivilegeDoctorCheckStatus::Pass,
            Some(false) => PrivilegeDoctorCheckStatus::Fail,
            None => PrivilegeDoctorCheckStatus::NotApplicable,
        },
        "helper root ownership check",
        root_owned
            .map(|value| vec![format!("root_owned={value}")])
            .unwrap_or_default(),
    ));
    checks.push(check(
        "helper.not_world_writable",
        match world_writable {
            Some(false) => PrivilegeDoctorCheckStatus::Pass,
            Some(true) => PrivilegeDoctorCheckStatus::Fail,
            None => PrivilegeDoctorCheckStatus::NotApplicable,
        },
        "helper world-writable mode check",
        world_writable
            .map(|value| vec![format!("world_writable={value}")])
            .unwrap_or_default(),
    ));
    checks.push(check(
        "helper.version",
        if helper_version.is_some() {
            PrivilegeDoctorCheckStatus::Pass
        } else if helper_installed {
            PrivilegeDoctorCheckStatus::Warning
        } else {
            PrivilegeDoctorCheckStatus::NotApplicable
        },
        "helper version command check",
        helper_version
            .clone()
            .map(|value| vec![format!("version={value}")])
            .unwrap_or_default(),
    ));
    checks.push(check(
        "sudo.non_interactive",
        if sudo_non_interactive_available {
            PrivilegeDoctorCheckStatus::Pass
        } else {
            PrivilegeDoctorCheckStatus::Fail
        },
        "sudo -n helper execution check",
        vec![format!("sudo -n {DEFAULT_PRIV_HELPER} --version")],
    ));

    let status = if helper_installed
        && root_owned == Some(true)
        && world_writable == Some(false)
        && sudo_non_interactive_available
    {
        PrivilegeDoctorStatus::Ready
    } else if helper_installed && helper_version.is_some() {
        PrivilegeDoctorStatus::Degraded
    } else {
        PrivilegeDoctorStatus::OperatorSetupRequired
    };
    let next_action = match status {
        PrivilegeDoctorStatus::Ready => {
            "privileged helper is ready for non-interactive typed apply/restore".to_string()
        }
        PrivilegeDoctorStatus::Degraded => {
            "repair helper ownership/mode or sudoers before Agent control execution".to_string()
        }
        PrivilegeDoctorStatus::OperatorSetupRequired => {
            "run adc-lab privilege install-plan and complete operator setup".to_string()
        }
        PrivilegeDoctorStatus::UnsupportedTarget => unreachable!(),
    };

    PrivilegeDoctorReport {
        schema_version: "lab.privilege_doctor.v1".to_string(),
        target_id,
        helper_path,
        helper_installed,
        root_owned,
        world_writable,
        sudo_non_interactive_available,
        helper_version,
        status,
        checks,
        next_action,
        time_unix_ms: now_unix_ms(),
    }
}

pub fn privilege_install_plan(target_id: String, helper_bin: Option<&Path>) -> PrivilegeSetupPlan {
    let helper_source = helper_bin
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "./adc-lab-priv-helper".to_string());
    PrivilegeSetupPlan {
        schema_version: "lab.privilege_setup_plan.v1".to_string(),
        plan_id: new_id("PRIV-PLAN"),
        target_id,
        plan_kind: PrivilegeSetupPlanKind::Install,
        helper_path: DEFAULT_PRIV_HELPER.to_string(),
        operator_steps: vec![
            "Verify the helper binary came from a checked release asset or local build."
                .to_string(),
            "Install the helper as root-owned executable at the fixed adc-lab helper path."
                .to_string(),
            "Optionally add a lab-target-only sudoers rule for the fixed helper path.".to_string(),
            "Run adc-lab privilege doctor --target local before Agent control execution."
                .to_string(),
        ],
        commands: vec![
            format!("sudo install -o root -g root -m 0755 {helper_source} {DEFAULT_PRIV_HELPER}"),
            "sudo visudo -f /etc/sudoers.d/adc-lab".to_string(),
        ],
        verification_commands: vec![
            "adc-lab privilege doctor --target local".to_string(),
            format!("sudo -n {DEFAULT_PRIV_HELPER} --version"),
        ],
        warnings: vec![
            "This plan is instructions only; adc-lab does not install privileged files."
                .to_string(),
            "sudoers, if used, must be limited to the fixed adc-lab helper path on lab targets."
                .to_string(),
        ],
        time_unix_ms: now_unix_ms(),
    }
}

pub fn privilege_uninstall_plan(target_id: String) -> PrivilegeSetupPlan {
    PrivilegeSetupPlan {
        schema_version: "lab.privilege_setup_plan.v1".to_string(),
        plan_id: new_id("PRIV-PLAN"),
        target_id,
        plan_kind: PrivilegeSetupPlanKind::Uninstall,
        helper_path: DEFAULT_PRIV_HELPER.to_string(),
        operator_steps: vec![
            "Confirm no approved control operation is in progress.".to_string(),
            "Remove the optional sudoers entry if it was installed.".to_string(),
            "Remove the fixed helper binary.".to_string(),
            "Run adc-lab privilege doctor --target local and expect operator setup required."
                .to_string(),
        ],
        commands: vec![
            "sudo rm -f /etc/sudoers.d/adc-lab".to_string(),
            format!("sudo rm -f {DEFAULT_PRIV_HELPER}"),
        ],
        verification_commands: vec![
            "adc-lab privilege doctor --target local".to_string(),
            format!("test ! -e {DEFAULT_PRIV_HELPER}"),
        ],
        warnings: vec![
            "Keep the release asset or source binary if reinstall may be needed.".to_string(),
            "This plan is instructions only; adc-lab does not remove privileged files.".to_string(),
        ],
        time_unix_ms: now_unix_ms(),
    }
}

fn option_a_provider() -> PrivilegeProviderDescriptor {
    PrivilegeProviderDescriptor {
        provider_id: OPTION_A_PROVIDER_ID.to_string(),
        provider_kind: PrivilegeProviderKind::SudoHelperOptionA,
        availability: PrivilegeProviderAvailability::Active,
        transport: PrivilegeProviderTransport::SudoExec,
        endpoint: DEFAULT_PRIV_HELPER.to_string(),
        root_boundary:
            "root-owned helper invoked through sudo; no shell or arbitrary command input"
                .to_string(),
        operations_allowed: vec![CPUFREQ_SET_GOVERNOR.to_string()],
        approval_required: true,
        audit_required: true,
        restore_required: true,
        default_enabled: true,
        safety_notes: vec![
            "controller CLI has no public helper path override".to_string(),
            "privileged apply and restore remain local-target only in this MVP".to_string(),
        ],
    }
}

fn option_b_provider() -> PrivilegeProviderDescriptor {
    PrivilegeProviderDescriptor {
        provider_id: OPTION_B_PROVIDER_ID.to_string(),
        provider_kind: PrivilegeProviderKind::SystemdUnixSocketOptionB,
        availability: PrivilegeProviderAvailability::PlannedDisabled,
        transport: PrivilegeProviderTransport::UnixSocket,
        endpoint: OPTION_B_PLANNED_SOCKET.to_string(),
        root_boundary:
            "planned root-owned provider over a bounded Unix socket; not installed or started in PR10"
                .to_string(),
        operations_allowed: Vec::new(),
        approval_required: true,
        audit_required: true,
        restore_required: true,
        default_enabled: false,
        safety_notes: vec![
            "no systemd unit, socket listener, or daemon is installed by PR10".to_string(),
            "not an active privileged transport and not evidence of remote privileged apply"
                .to_string(),
        ],
    }
}

fn check(
    check_id: &str,
    status: PrivilegeDoctorCheckStatus,
    summary: &str,
    evidence: Vec<String>,
) -> PrivilegeDoctorCheck {
    PrivilegeDoctorCheck {
        check_id: check_id.to_string(),
        status,
        summary: summary.to_string(),
        evidence,
    }
}

#[cfg(unix)]
fn root_owned(metadata: &fs::Metadata) -> Option<bool> {
    Some(metadata.uid() == 0)
}

#[cfg(not(unix))]
fn root_owned(_metadata: &fs::Metadata) -> Option<bool> {
    None
}

#[cfg(unix)]
fn world_writable(metadata: &fs::Metadata) -> Option<bool> {
    Some(metadata.mode() & 0o002 != 0)
}

#[cfg(not(unix))]
fn world_writable(_metadata: &fs::Metadata) -> Option<bool> {
    None
}

fn helper_version(helper: &Path) -> Option<String> {
    let output = Command::new(helper).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let info: BuildInfo = serde_json::from_slice(&output.stdout).ok()?;
    Some(info.version)
}

fn sudo_non_interactive_version(helper: &Path) -> Option<String> {
    let output = Command::new("sudo")
        .arg("-n")
        .arg(helper)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let info: BuildInfo = serde_json::from_slice(&output.stdout).ok()?;
    Some(info.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn privilege_provider_status_keeps_option_a_active_and_option_b_disabled() {
        let status = privilege_provider_status("local-target".to_string());
        assert_eq!(status.schema_version, "lab.privilege_provider_status.v1");
        assert_eq!(status.active_provider_id, OPTION_A_PROVIDER_ID);

        let option_a = status
            .providers
            .iter()
            .find(|provider| provider.provider_id == OPTION_A_PROVIDER_ID)
            .unwrap();
        assert_eq!(option_a.availability, PrivilegeProviderAvailability::Active);
        assert_eq!(option_a.transport, PrivilegeProviderTransport::SudoExec);
        assert_eq!(option_a.endpoint, DEFAULT_PRIV_HELPER);
        assert_eq!(option_a.operations_allowed, vec![CPUFREQ_SET_GOVERNOR]);
        assert!(option_a.default_enabled);

        let option_b = status
            .providers
            .iter()
            .find(|provider| provider.provider_id == OPTION_B_PROVIDER_ID)
            .unwrap();
        assert_eq!(
            option_b.availability,
            PrivilegeProviderAvailability::PlannedDisabled
        );
        assert_eq!(option_b.transport, PrivilegeProviderTransport::UnixSocket);
        assert!(option_b.operations_allowed.is_empty());
        assert!(!option_b.default_enabled);
    }

    #[test]
    fn privilege_install_plan_is_instruction_only_for_fixed_helper() {
        let plan = privilege_install_plan("local-target".to_string(), None);
        assert_eq!(plan.schema_version, "lab.privilege_setup_plan.v1");
        assert_eq!(plan.plan_kind, PrivilegeSetupPlanKind::Install);
        assert_eq!(plan.helper_path, DEFAULT_PRIV_HELPER);
        assert!(plan.commands.iter().any(|command| command
            .contains("sudo install -o root -g root -m 0755 ./adc-lab-priv-helper")));
        assert!(plan
            .verification_commands
            .iter()
            .any(|command| command.contains("sudo -n")));
    }

    #[test]
    fn privilege_doctor_remote_target_is_unsupported_without_probe() {
        let report = privilege_doctor("target55".to_string(), false);
        assert_eq!(report.schema_version, "lab.privilege_doctor.v1");
        assert_eq!(report.status, PrivilegeDoctorStatus::UnsupportedTarget);
        assert!(!report.sudo_non_interactive_available);
    }
}
