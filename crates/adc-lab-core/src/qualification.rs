use crate::contracts::{QualificationStatus, ToolQualification};
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
        status,
        evidence_accepted: false,
        dry_run_required: manifest.qualification.dry_run_required,
        checks,
        limitations,
        time_unix_ms: now_unix_ms(),
    })
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
}
