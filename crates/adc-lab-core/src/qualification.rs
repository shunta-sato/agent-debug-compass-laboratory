use crate::contracts::{
    PrivilegeLevel, QualificationStatus, ToolCategory, ToolInfo, ToolQualification,
    ToolQualificationSummary, ToolQualificationSummaryEntry, ToolSource, ToolchainInventory,
};
use crate::ids::now_unix_ms;
use crate::{LabError, LabResult};
use serde::{Deserialize, Serialize};

pub const AGENT_ADAPTER_DURATION_SECONDS_MAX: u64 = 30;
pub const AGENT_ADAPTER_OUTPUT_BYTES_MAX: u64 = 1024 * 1024;

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ToolQualificationEvidence {
    pub tool_version: String,
    pub tool_sha256: String,
    pub output_schema_ref: String,
    pub dry_run_ref: String,
    pub manual_comparison_ref: String,
    pub static_safety_review_ref: String,
    pub validated_output_bytes: u64,
}

pub fn qualify_tool(manifest: ToolManifest) -> LabResult<ToolQualification> {
    qualify_tool_with_evidence(manifest, None)
}

pub fn qualify_tool_with_evidence(
    manifest: ToolManifest,
    evidence: Option<ToolQualificationEvidence>,
) -> LabResult<ToolQualification> {
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
    if manifest.failure_modes.is_empty() {
        return Err(LabError::Validation(
            "at least one failure mode must be declared".to_string(),
        ));
    }
    let category = tool_category_from_manifest_kind(&manifest.kind);
    let privilege = if manifest.requires_privilege {
        PrivilegeLevel::SudoHelper
    } else {
        PrivilegeLevel::None
    };
    let mut limitations = Vec::new();
    if let Some(evidence) = evidence {
        validate_agent_tool_evidence(&manifest, &evidence)?;
        checks.extend([
            "dry_run_evidence_recorded".to_string(),
            "manual_comparison_recorded".to_string(),
            "tool_version_recorded".to_string(),
            "tool_sha256_recorded".to_string(),
            "output_schema_ref_recorded".to_string(),
            "static_safety_review_recorded".to_string(),
            "output_bytes_within_bounds".to_string(),
        ]);
        let refs = vec![
            evidence.output_schema_ref.clone(),
            evidence.dry_run_ref.clone(),
            evidence.manual_comparison_ref.clone(),
            evidence.static_safety_review_ref.clone(),
        ];
        if agent_adapter_scope_is_qualifiable(&manifest, &category, &privilege) {
            checks.push("agent_adapter_scope_allowlist".to_string());
            limitations.push(
                "qualified only for the declared bounded non-privileged adapter output scope"
                    .to_string(),
            );
            limitations.push(
                "does not qualify control, restore, load, production readiness, battery, flash, thermal safety, or low-overhead claims"
                    .to_string(),
            );
            return Ok(ToolQualification {
                schema_version: "lab.tool_qualification.v1".to_string(),
                tool_id: manifest.tool_id,
                category,
                privilege,
                source: ToolSource::AgentCreated,
                available: true,
                status: QualificationStatus::Qualified,
                evidence_accepted: true,
                dry_run_required: manifest.qualification.dry_run_required,
                qualification_scope: "agent_created_bounded_observation_adapter".to_string(),
                tool_version: Some(evidence.tool_version),
                tool_sha256: Some(evidence.tool_sha256),
                output_schema_ref: Some(evidence.output_schema_ref),
                dry_run_ref: Some(evidence.dry_run_ref),
                manual_comparison_ref: Some(evidence.manual_comparison_ref),
                static_safety_review_ref: Some(evidence.static_safety_review_ref),
                validated_output_bytes: Some(evidence.validated_output_bytes),
                evidence_refs: refs,
                checks,
                limitations,
                reason: "agent-created bounded adapter has required qualification evidence"
                    .to_string(),
                time_unix_ms: now_unix_ms(),
            });
        }
        checks.push("agent_adapter_scope_refused".to_string());
        limitations.push(
            "PR9 only qualifies non-state-writing, non-privileged observation/probe/report-normalizer/health-check adapters".to_string(),
        );
        return Ok(ToolQualification {
            schema_version: "lab.tool_qualification.v1".to_string(),
            tool_id: manifest.tool_id,
            category,
            privilege,
            source: ToolSource::AgentCreated,
            available: false,
            status: QualificationStatus::AgentCreatedUnqualified,
            evidence_accepted: false,
            dry_run_required: manifest.qualification.dry_run_required,
            qualification_scope: "agent_created_scope_refused".to_string(),
            tool_version: Some(evidence.tool_version),
            tool_sha256: Some(evidence.tool_sha256),
            output_schema_ref: Some(evidence.output_schema_ref),
            dry_run_ref: Some(evidence.dry_run_ref),
            manual_comparison_ref: Some(evidence.manual_comparison_ref),
            static_safety_review_ref: Some(evidence.static_safety_review_ref),
            validated_output_bytes: Some(evidence.validated_output_bytes),
            evidence_refs: refs,
            checks,
            limitations,
            reason:
                "agent-created tool category or privilege requires a future qualification workflow"
                    .to_string(),
            time_unix_ms: now_unix_ms(),
        });
    }

    checks.push("qualification_evidence_missing".to_string());
    limitations.push(
        "agent-created or external tools are not accepted as evidence until dry-run, comparison evidence, version/hash capture, and output validation are recorded"
            .to_string(),
    );
    Ok(ToolQualification {
        schema_version: "lab.tool_qualification.v1".to_string(),
        tool_id: manifest.tool_id,
        category,
        privilege,
        source: ToolSource::AgentCreated,
        available: false,
        status: QualificationStatus::AgentCreatedUnqualified,
        evidence_accepted: false,
        dry_run_required: manifest.qualification.dry_run_required,
        qualification_scope: "agent_created_manifest_only".to_string(),
        tool_version: None,
        tool_sha256: None,
        output_schema_ref: None,
        dry_run_ref: None,
        manual_comparison_ref: None,
        static_safety_review_ref: None,
        validated_output_bytes: None,
        evidence_refs: vec![],
        checks,
        limitations,
        reason: "agent-created tool manifest lacks qualification evidence".to_string(),
        time_unix_ms: now_unix_ms(),
    })
}

fn validate_agent_tool_evidence(
    manifest: &ToolManifest,
    evidence: &ToolQualificationEvidence,
) -> LabResult<()> {
    if evidence.tool_version.trim().is_empty() {
        return Err(LabError::Validation("tool_version is required".to_string()));
    }
    if !valid_sha256_digest(&evidence.tool_sha256) {
        return Err(LabError::Validation(
            "tool_sha256 must be sha256:<64 lowercase hex chars>".to_string(),
        ));
    }
    for (label, reference) in [
        ("output_schema_ref", &evidence.output_schema_ref),
        ("dry_run_ref", &evidence.dry_run_ref),
        ("manual_comparison_ref", &evidence.manual_comparison_ref),
        (
            "static_safety_review_ref",
            &evidence.static_safety_review_ref,
        ),
    ] {
        if !reference.starts_with("artifact://lab/runs/") {
            return Err(LabError::Validation(format!(
                "{label} must be an artifact://lab/runs/... ref"
            )));
        }
    }
    if evidence.validated_output_bytes == 0 {
        return Err(LabError::Validation(
            "validated_output_bytes must be non-zero".to_string(),
        ));
    }
    if evidence.validated_output_bytes > manifest.bounded.output_bytes_max {
        return Err(LabError::Validation(
            "validated output exceeds manifest output_bytes_max".to_string(),
        ));
    }
    if manifest.bounded.duration_seconds_max > AGENT_ADAPTER_DURATION_SECONDS_MAX {
        return Err(LabError::Validation(format!(
            "agent-created adapter duration_seconds_max must be <= {AGENT_ADAPTER_DURATION_SECONDS_MAX}"
        )));
    }
    if manifest.bounded.output_bytes_max > AGENT_ADAPTER_OUTPUT_BYTES_MAX {
        return Err(LabError::Validation(format!(
            "agent-created adapter output_bytes_max must be <= {AGENT_ADAPTER_OUTPUT_BYTES_MAX}"
        )));
    }
    Ok(())
}

fn valid_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
}

fn agent_adapter_scope_is_qualifiable(
    manifest: &ToolManifest,
    category: &ToolCategory,
    privilege: &PrivilegeLevel,
) -> bool {
    !manifest.writes_target_state
        && !manifest.requires_privilege
        && manifest.qualification.dry_run_required
        && manifest.qualification.compare_with_manual_sample
        && *privilege == PrivilegeLevel::None
        && matches!(
            category,
            ToolCategory::Observation
                | ToolCategory::Probe
                | ToolCategory::ReportNormalizer
                | ToolCategory::HealthCheck
        )
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
    let mut tools = Vec::new();

    for (report, artifact_ref) in reports {
        qualification_refs.push(artifact_ref.clone());
        tools.push(ToolQualificationSummaryEntry {
            tool_id: report.tool_id.clone(),
            status: summary_status(report),
            evidence_accepted: report.evidence_accepted,
        });
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
        tools,
        qualification_refs,
        evidence_accepted_tool_ids,
        evidence_rejected_tool_ids,
        missing_tool_ids,
        time_unix_ms: now_unix_ms(),
    }
}

fn summary_status(report: &ToolQualification) -> String {
    if report.status == QualificationStatus::Builtin
        && report.category == ToolCategory::Load
        && report.privilege == PrivilegeLevel::User
    {
        "builtin_bounded".to_string()
    } else {
        serde_json::to_string(&report.status)
            .unwrap_or_else(|_| "\"unknown\"".to_string())
            .trim_matches('"')
            .to_string()
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
    } else if is_builtin_bounded_load_evidence_tool(tool) {
        checks.push("builtin_bounded_load_allowlist".to_string());
        checks.push("pr5_safety_monitor_contract_present".to_string());
        (
            ToolSource::Builtin,
            QualificationStatus::Builtin,
            true,
            false,
            "builtin bounded CPU load is accepted for lab.load_plan.v1 and lab.load_result.v1 evidence".to_string(),
            vec![
                "accepted only for explicit Tier 1 bounded CPU load results with duration, worker, thermal, operator-abort, and safety-monitor evidence".to_string(),
                "does not support sustained thermal, battery, flash, latency, low-overhead, or production readiness claims".to_string(),
            ],
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
    } else {
        (
            ToolSource::Builtin,
            QualificationStatus::Refused,
            false,
            true,
            "tool is not in the current evidence allowlist".to_string(),
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
        qualification_scope: if evidence_accepted {
            "builtin_allowlist".to_string()
        } else {
            "inventory_policy".to_string()
        },
        tool_version: None,
        tool_sha256: None,
        output_schema_ref: None,
        dry_run_ref: None,
        manual_comparison_ref: None,
        static_safety_review_ref: None,
        validated_output_bytes: None,
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

fn is_builtin_bounded_load_evidence_tool(tool: &ToolInfo) -> bool {
    tool.available
        && tool.tool_id == "adc-lab-builtin-cpu-load"
        && tool.category == ToolCategory::Load
        && tool.qualification == QualificationStatus::Builtin
        && tool.privilege == PrivilegeLevel::User
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

    fn complete_evidence() -> ToolQualificationEvidence {
        ToolQualificationEvidence {
            tool_version: "0.1.0".to_string(),
            tool_sha256: format!("sha256:{}", "a".repeat(64)),
            output_schema_ref:
                "artifact://lab/runs/LAB-RUN-001/tools/agent_tool.output_schema.json".to_string(),
            dry_run_ref: "artifact://lab/runs/LAB-RUN-001/tools/agent_tool.dry_run.json"
                .to_string(),
            manual_comparison_ref:
                "artifact://lab/runs/LAB-RUN-001/tools/agent_tool.manual_comparison.json"
                    .to_string(),
            static_safety_review_ref:
                "artifact://lab/runs/LAB-RUN-001/tools/agent_tool.static_safety_review.txt"
                    .to_string(),
            validated_output_bytes: 128,
        }
    }

    #[test]
    fn contract_validation_manifest_only_never_qualifies_tool() {
        let report = qualify_tool(manifest(false, false)).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
        assert_eq!(report.source, ToolSource::AgentCreated);
        assert_eq!(report.qualification_scope, "agent_created_manifest_only");
        assert!(report.tool_version.is_none());
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
    fn contract_validation_complete_agent_observation_adapter_evidence_qualifies() {
        let report =
            qualify_tool_with_evidence(manifest(true, true), Some(complete_evidence())).unwrap();
        assert_eq!(report.status, QualificationStatus::Qualified);
        assert!(report.evidence_accepted);
        assert!(report.available);
        assert_eq!(
            report.qualification_scope,
            "agent_created_bounded_observation_adapter"
        );
        assert_eq!(report.tool_version.as_deref(), Some("0.1.0"));
        assert_eq!(report.validated_output_bytes, Some(128));
        assert!(report
            .evidence_refs
            .iter()
            .all(|artifact| artifact.starts_with("artifact://lab/runs/")));
        assert!(report
            .checks
            .contains(&"agent_adapter_scope_allowlist".to_string()));
    }

    #[test]
    fn contract_validation_agent_control_adapter_evidence_stays_unqualified() {
        let mut manifest = manifest(true, true);
        manifest.kind = "control".to_string();
        manifest.writes_target_state = true;
        let report = qualify_tool_with_evidence(manifest, Some(complete_evidence())).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
        assert_eq!(report.qualification_scope, "agent_created_scope_refused");
        assert!(report
            .checks
            .contains(&"agent_adapter_scope_refused".to_string()));
    }

    #[test]
    fn contract_validation_agent_load_adapter_evidence_stays_unqualified() {
        let mut manifest = manifest(true, true);
        manifest.kind = "load".to_string();
        let report = qualify_tool_with_evidence(manifest, Some(complete_evidence())).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
    }

    #[test]
    fn contract_validation_agent_adapter_manifest_must_require_dry_run_and_comparison() {
        let report =
            qualify_tool_with_evidence(manifest(false, false), Some(complete_evidence())).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
    }

    #[test]
    fn contract_validation_privileged_agent_adapter_evidence_stays_unqualified() {
        let mut manifest = manifest(true, true);
        manifest.requires_privilege = true;
        let report = qualify_tool_with_evidence(manifest, Some(complete_evidence())).unwrap();
        assert_eq!(report.status, QualificationStatus::AgentCreatedUnqualified);
        assert!(!report.evidence_accepted);
        assert_eq!(report.privilege, PrivilegeLevel::SudoHelper);
    }

    #[test]
    fn contract_validation_agent_adapter_evidence_requires_valid_sha256() {
        let mut evidence = complete_evidence();
        evidence.tool_sha256 = "sha256:BAD".to_string();
        let error = qualify_tool_with_evidence(manifest(true, true), Some(evidence)).unwrap_err();
        assert!(error.to_string().contains("tool_sha256"));
    }

    #[test]
    fn contract_validation_agent_adapter_evidence_requires_logical_refs() {
        let mut evidence = complete_evidence();
        evidence.dry_run_ref = "/tmp/dry-run.json".to_string();
        let error = qualify_tool_with_evidence(manifest(true, true), Some(evidence)).unwrap_err();
        assert!(error.to_string().contains("dry_run_ref"));
    }

    #[test]
    fn contract_validation_agent_adapter_bounds_are_capped() {
        let mut manifest = manifest(true, true);
        manifest.bounded.duration_seconds_max = AGENT_ADAPTER_DURATION_SECONDS_MAX + 1;
        let error = qualify_tool_with_evidence(manifest, Some(complete_evidence())).unwrap_err();
        assert!(error.to_string().contains("duration_seconds_max"));
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
    fn contract_validation_builtin_cpu_load_is_bounded_evidence_source() {
        let report = qualify_tool_info(
            &ToolInfo {
                tool_id: "adc-lab-builtin-cpu-load".to_string(),
                category: ToolCategory::Load,
                available: true,
                privilege: PrivilegeLevel::User,
                qualification: QualificationStatus::Builtin,
            },
            Some("artifact://lab/runs/LAB-RUN-001/toolchain/toolchain_inventory.json".to_string()),
        );
        assert_eq!(report.status, QualificationStatus::Builtin);
        assert!(report.evidence_accepted);
        assert!(report
            .checks
            .contains(&"pr5_safety_monitor_contract_present".to_string()));
        assert!(report
            .limitations
            .iter()
            .any(|limit| limit.contains("production readiness")));
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
