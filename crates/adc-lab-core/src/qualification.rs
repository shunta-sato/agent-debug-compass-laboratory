use crate::contracts::{
    PrivilegeLevel, QualificationStatus, ToolCategory, ToolInfo, ToolQualification,
    ToolQualificationSummary, ToolSource, ToolchainInventory,
};
use crate::ids::now_unix_ms;
use crate::{LabError, LabResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolManifest {
    pub tool_id: String,
    pub kind: String,
    pub writes_target_state: bool,
    pub requires_privilege: bool,
    pub bounded: ToolBounds,
    pub expected_overhead: String,
    pub failure_modes: Vec<String>,
    pub qualification: ToolManifestQualification,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolBounds {
    pub duration_seconds_max: u64,
    pub output_bytes_max: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolManifestQualification {
    pub dry_run_required: bool,
    pub compare_with_manual_sample: bool,
}

pub fn qualify_tool(manifest: ToolManifest) -> LabResult<ToolQualification> {
    if manifest.tool_id.trim().is_empty() {
        return Err(LabError::Validation("tool_id is required".to_string()));
    }
    if manifest.bounded.duration_seconds_max == 0 || manifest.bounded.output_bytes_max == 0 {
        return Err(LabError::Validation(
            "tool bounds must include non-zero duration and output bytes".to_string(),
        ));
    }
    let mut checks = vec![
        "manifest_parsed".to_string(),
        "bounds_present".to_string(),
        "failure_modes_declared".to_string(),
    ];
    let mut limitations = Vec::new();
    checks.push("qualification_evidence_missing".to_string());
    limitations.push(
        "agent-created or external tools are not accepted as evidence until dry-run, comparison evidence, version/hash capture, and output validation are recorded"
            .to_string(),
    );
    let status = QualificationStatus::AgentCreatedUnqualified;
    Ok(ToolQualification {
        schema_version: "lab.tool_qualification.v1".to_string(),
        tool_id: manifest.tool_id,
        category: tool_category_from_manifest_kind(&manifest.kind),
        privilege: if manifest.requires_privilege {
            PrivilegeLevel::SudoHelper
        } else {
            PrivilegeLevel::None
        },
        source: ToolSource::AgentCreated,
        available: false,
        status,
        evidence_accepted: false,
        dry_run_required: manifest.qualification.dry_run_required,
        evidence_refs: vec![],
        checks,
        limitations,
        reason: "agent-created tool manifest lacks qualification evidence".to_string(),
        time_unix_ms: now_unix_ms(),
    })
}

pub fn qualify_toolchain_inventory(
    inventory: &ToolchainInventory,
    inventory_ref: Option<String>,
) -> Vec<ToolQualification> {
    inventory
        .tools
        .iter()
        .map(|tool| qualify_tool_info(tool, inventory_ref.clone()))
        .collect()
}

pub fn summarize_tool_qualifications(
    target_id: String,
    reports: &[(ToolQualification, String)],
) -> ToolQualificationSummary {
    let mut qualification_refs = Vec::new();
    let mut evidence_accepted_tool_ids = Vec::new();
    let mut evidence_rejected_tool_ids = Vec::new();
    let mut missing_tool_ids = Vec::new();

    for (report, artifact_ref) in reports {
        qualification_refs.push(artifact_ref.clone());
        if report.evidence_accepted {
            evidence_accepted_tool_ids.push(report.tool_id.clone());
        } else {
            evidence_rejected_tool_ids.push(report.tool_id.clone());
        }
        if !report.available {
            missing_tool_ids.push(report.tool_id.clone());
        }
    }

    ToolQualificationSummary {
        schema_version: "lab.tool_qualification_summary.v1".to_string(),
        target_id,
        qualification_refs,
        evidence_accepted_tool_ids,
        evidence_rejected_tool_ids,
        missing_tool_ids,
        time_unix_ms: now_unix_ms(),
    }
}

pub fn qualify_tool_info(tool: &ToolInfo, inventory_ref: Option<String>) -> ToolQualification {
    let mut checks = vec![
        "toolchain_inventory_present".to_string(),
        "availability_recorded".to_string(),
        "privilege_recorded".to_string(),
        "category_recorded".to_string(),
    ];
    let evidence_refs = inventory_ref.into_iter().collect::<Vec<_>>();

    let (source, status, evidence_accepted, dry_run_required, reason, limitations) = if !tool
        .available
    {
        (
            ToolSource::Missing,
            QualificationStatus::Refused,
            false,
            false,
            "tool is unavailable on the target".to_string(),
            vec!["missing tools cannot be used as evidence sources".to_string()],
        )
    } else if is_builtin_read_only_evidence_tool(tool) {
        checks.push("builtin_read_only_allowlist".to_string());
        (
            ToolSource::Builtin,
            QualificationStatus::Builtin,
            true,
            false,
            "builtin read-only tool is accepted as an evidence source".to_string(),
            Vec::new(),
        )
    } else if tool.qualification == QualificationStatus::ExternalUnqualified {
        (
            ToolSource::External,
            QualificationStatus::ExternalUnqualified,
            false,
            true,
            "external tool requires qualification evidence before use".to_string(),
            vec![
                "version/hash capture is required".to_string(),
                "dry-run and output validation are required".to_string(),
            ],
        )
    } else if matches!(
        tool.category,
        ToolCategory::ObservationControl | ToolCategory::Control
    ) || matches!(
        tool.privilege,
        PrivilegeLevel::SudoHelper | PrivilegeLevel::Root
    ) {
        (
                ToolSource::Builtin,
                QualificationStatus::NeedsControlTest,
                false,
                true,
                "control-capable or privileged tool requires control qualification".to_string(),
                vec![
                    "approval, restore, and verification evidence are required before control output can support claims".to_string(),
                ],
            )
    } else if tool.category == ToolCategory::Load {
        (
                ToolSource::Builtin,
                QualificationStatus::Refused,
                false,
                true,
                "load tool evidence is deferred until bounded load safety evidence exists".to_string(),
                vec![
                    "bounded load and safety monitor evidence are required before load results can support claims".to_string(),
                ],
            )
    } else {
        (
            ToolSource::Builtin,
            QualificationStatus::Refused,
            false,
            true,
            "tool is not in the PR3 read-only evidence allowlist".to_string(),
            vec!["explicit qualification policy is required".to_string()],
        )
    };

    ToolQualification {
        schema_version: "lab.tool_qualification.v1".to_string(),
        tool_id: tool.tool_id.clone(),
        category: tool.category.clone(),
        privilege: tool.privilege.clone(),
        source,
        available: tool.available,
        status,
        evidence_accepted,
        dry_run_required,
        evidence_refs,
        checks,
        limitations,
        reason,
        time_unix_ms: now_unix_ms(),
    }
}

fn is_builtin_read_only_evidence_tool(tool: &ToolInfo) -> bool {
    tool.available
        && tool.qualification == QualificationStatus::Builtin
        && tool.privilege == PrivilegeLevel::None
        && matches!(
            tool.category,
            ToolCategory::Observation
                | ToolCategory::Probe
                | ToolCategory::ReportNormalizer
                | ToolCategory::HealthCheck
        )
}

fn tool_category_from_manifest_kind(kind: &str) -> ToolCategory {
    match kind {
        "control" => ToolCategory::Control,
        "load" => ToolCategory::Load,
        "probe" => ToolCategory::Probe,
        "report_normalizer" => ToolCategory::ReportNormalizer,
        "restore" => ToolCategory::Restore,
        "health_check" => ToolCategory::HealthCheck,
        "observation_control" => ToolCategory::ObservationControl,
        _ => ToolCategory::Observation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(dry_run_required: bool, compare_with_manual_sample: bool) -> ToolManifest {
        ToolManifest {
            tool_id: "agent_tool".to_string(),
            kind: "observation".to_string(),
            writes_target_state: false,
            requires_privilege: false,
            bounded: ToolBounds {
                duration_seconds_max: 30,
                output_bytes_max: 1024,
            },
            expected_overhead: "low".to_string(),
            failure_modes: vec!["surface missing".to_string()],
            qualification: ToolManifestQualification {
                dry_run_required,
                compare_with_manual_sample,
            },
        }
    }

    #[test]
    fn contract_validation_manifest_only_never_qualifies_tool() {
        let report = qualify_tool(manifest(false, false)).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
        assert_eq!(report.source, ToolSource::AgentCreated);
        assert!(report
            .checks
            .contains(&"qualification_evidence_missing".to_string()));
    }

    #[test]
    fn contract_validation_required_qualification_evidence_stays_unqualified() {
        let report = qualify_tool(manifest(true, true)).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
    }

    #[test]
    fn contract_validation_builtin_read_only_tool_is_evidence_accepted() {
        let report = qualify_tool_info(
            &ToolInfo {
                tool_id: "linux.procfs".to_string(),
                category: ToolCategory::Observation,
                available: true,
                privilege: PrivilegeLevel::None,
                qualification: QualificationStatus::Builtin,
            },
            Some("artifact://lab/runs/LAB-RUN-001/toolchain/toolchain_inventory.json".to_string()),
        );
        assert_eq!(report.status, QualificationStatus::Builtin);
        assert!(report.evidence_accepted);
        assert_eq!(report.source, ToolSource::Builtin);
    }

    #[test]
    fn contract_validation_control_tool_is_not_evidence_accepted() {
        let report = qualify_tool_info(
            &ToolInfo {
                tool_id: "linux.cpufreq.sysfs".to_string(),
                category: ToolCategory::ObservationControl,
                available: true,
                privilege: PrivilegeLevel::SudoHelper,
                qualification: QualificationStatus::NeedsControlTest,
            },
            None,
        );
        assert_eq!(report.status, QualificationStatus::NeedsControlTest);
        assert!(!report.evidence_accepted);
    }

    #[test]
    fn contract_validation_missing_tool_is_refused() {
        let report = qualify_tool_info(
            &ToolInfo {
                tool_id: "stress-ng".to_string(),
                category: ToolCategory::Load,
                available: false,
                privilege: PrivilegeLevel::User,
                qualification: QualificationStatus::ExternalUnqualified,
            },
            None,
        );
        assert_eq!(report.status, QualificationStatus::Refused);
        assert_eq!(report.source, ToolSource::Missing);
        assert!(!report.evidence_accepted);
    }
}
