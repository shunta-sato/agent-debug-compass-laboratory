use crate::contracts::{
    PrivilegeProviderAvailability, PrivilegeProviderDescriptor, PrivilegeProviderKind,
    PrivilegeProviderStatus, PrivilegeProviderTransport,
};
use crate::control::{CPUFREQ_SET_GOVERNOR, DEFAULT_PRIV_HELPER};
use crate::ids::now_unix_ms;

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
}
