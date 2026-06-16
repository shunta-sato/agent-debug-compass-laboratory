use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTargetLocalEnvRequirement {
    pub name: String,
    pub value: String,
    pub required_when: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTargetLocalFailureDiagnostic {
    pub category: String,
    pub signal: String,
    pub operator_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowTargetLocalExecutionGuide {
    pub applies_to_execution_location: String,
    pub working_directory_policy: String,
    pub path_prepend: Vec<String>,
    pub env: Vec<WorkflowTargetLocalEnvRequirement>,
    pub argv_semantics: String,
    pub ssh_invocation_template: Vec<String>,
    pub failure_diagnostics: Vec<WorkflowTargetLocalFailureDiagnostic>,
}

pub(crate) fn target_local_execution_guide(
    target_is_ssh: bool,
    endpoint: &str,
) -> Option<WorkflowTargetLocalExecutionGuide> {
    if !target_is_ssh {
        return None;
    }

    Some(WorkflowTargetLocalExecutionGuide {
        applies_to_execution_location: "target_local".to_string(),
        working_directory_policy: "target_local_repository_root".to_string(),
        path_prepend: vec![
            "$HOME/.local/bin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
        ],
        env: vec![
            WorkflowTargetLocalEnvRequirement {
                name: "PATH".to_string(),
                value: "$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH".to_string(),
                required_when: "before executing target_local command_argv steps".to_string(),
            },
            WorkflowTargetLocalEnvRequirement {
                name: "ADC_LAB_TARGET_RUNNER".to_string(),
                value: "/home/<target-user>/.local/bin/adc-lab-target".to_string(),
                required_when: "controller-side ssh commands when default adc-lab-target is not on the non-interactive SSH PATH"
                    .to_string(),
            },
        ],
        argv_semantics: "preserve command_argv as ordered arguments; quote each remote arg independently and never concatenate argv into a shell script"
            .to_string(),
        ssh_invocation_template: vec![
            "ssh".to_string(),
            endpoint.to_string(),
            "env".to_string(),
            "PATH=$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin:$PATH".to_string(),
            "<command_argv entries, shell-quoted one-by-one>".to_string(),
        ],
        failure_diagnostics: vec![
            target_local_failure_diagnostic(
                "command_not_found",
                "ssh exit 127 or remote stderr reports adc-lab-target not found",
                "check ADC_LAB_TARGET_RUNNER and install location before treating evidence as missing",
            ),
            target_local_failure_diagnostic(
                "path_missing",
                "default adc-lab-target lookup fails while release installer path is expected under ~/.local/bin",
                "set ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target",
            ),
            target_local_failure_diagnostic(
                "permission_denied",
                "ssh exit 126 or remote stderr reports Permission denied",
                "fix target runner execute permission or SSH access; do not retry through a shell wrapper",
            ),
            target_local_failure_diagnostic(
                "helper_unavailable",
                "control refusal reason privileged_apply_requires_target_local_helper",
                "preserve refusal evidence; privileged apply/restore must remain target-local typed helper work",
            ),
            target_local_failure_diagnostic(
                "version_skew",
                "report.run_validation gap blocked_by_version_skew or version_skew_override_still_blocked",
                "record the skew; full-set measured and selection-ready claims remain blocked",
            ),
        ],
    })
}

fn target_local_failure_diagnostic(
    category: &str,
    signal: &str,
    operator_action: &str,
) -> WorkflowTargetLocalFailureDiagnostic {
    WorkflowTargetLocalFailureDiagnostic {
        category: category.to_string(),
        signal: signal.to_string(),
        operator_action: operator_action.to_string(),
    }
}
