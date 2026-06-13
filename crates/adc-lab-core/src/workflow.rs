use crate::build_info::build_info;
use crate::contracts::BuildInfo;
use crate::evidence::{Artifact, DataQuality, DataQualityLevel, Kind, Status};
use crate::ids::{new_id, now_unix_ms};
use crate::{LabError, LabResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET: &str =
    "target-operating-contract-fullset";
pub const WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023: &str =
    "target-operating-contract-fullset.v0.2.3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowRecommendationMode {
    OfflineRecommendation,
    CapabilityCheckedRecommendation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowVersionPolicy {
    pub claim_producing_run_requires_no_skew: bool,
    pub allow_skew_override_records_gap: bool,
    pub skew_blocks_fullset_measured_claims: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowEvidencePolicy {
    pub recommendation_is_target_measurement_evidence: bool,
    pub raw_primitives_are_claim_producing: bool,
    pub causal_linkage_required: bool,
    pub forbidden_linkage_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowRecommendationPayload {
    pub goal: String,
    pub workflow_id: String,
    pub recommendation_mode: WorkflowRecommendationMode,
    pub controller_adc_lab: BuildInfo,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub source_of_truth_chain: Vec<String>,
    pub must_use: Vec<String>,
    pub must_not_use_for_claims: Vec<String>,
    pub forbidden_patterns: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub version_policy: WorkflowVersionPolicy,
    pub evidence_policy: WorkflowEvidencePolicy,
    pub next_step: String,
}

pub struct WorkflowRecommendationInput {
    pub run_id: String,
    pub goal: String,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub recommendation_mode: WorkflowRecommendationMode,
}

pub fn validate_workflow_goal(goal: &str) -> LabResult<()> {
    if goal == WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET {
        Ok(())
    } else {
        Err(LabError::Validation(format!(
            "unsupported workflow goal {}; expected {}",
            goal, WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET
        )))
    }
}

pub fn target_operating_contract_workflow_recommendation(
    input: WorkflowRecommendationInput,
) -> LabResult<Artifact<WorkflowRecommendationPayload>> {
    validate_workflow_goal(&input.goal)?;
    let mut artifact = Artifact::new(
        Kind::WorkflowRecommendation,
        new_id("WORKFLOW-RECOMMENDATION"),
        input.run_id,
        input.target_id.clone(),
        Status::NotApplicable {
            reason: "workflow authority artifact; not target measurement evidence".to_string(),
        },
        WorkflowRecommendationPayload {
            goal: input.goal,
            workflow_id: WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023.to_string(),
            recommendation_mode: input.recommendation_mode,
            controller_adc_lab: build_info("adc-lab"),
            target: input.target,
            target_id: input.target_id,
            target_class: input.target_class,
            source_of_truth_chain: vec![
                "workflow.recommendation".to_string(),
                "workflow.collect_plan".to_string(),
                "report.run_validation".to_string(),
                "report.operating_contract".to_string(),
                "report.suitability".to_string(),
                "report.constraints".to_string(),
            ],
            must_use: vec![
                "adc-lab collect plan or equivalent workflow.collect_plan artifact".to_string(),
                "adc-lab control governor-sweep prepare/approve/run for governor evidence"
                    .to_string(),
                "adc-lab report validate-run before controlled-governor operating-contract claims"
                    .to_string(),
            ],
            must_not_use_for_claims: vec![
                "manual plan/approval/lease discovery by filename order".to_string(),
                "raw primitive control artifacts without report.run_validation".to_string(),
                "static v0.2.1 target-operating-contract full-set harness".to_string(),
            ],
            forbidden_patterns: vec![
                "find PLAN-*.json | sort".to_string(),
                "find APPROVAL-*.json | sort".to_string(),
                "find LEASE-*.json | sort".to_string(),
                "tail -n 1 for artifact selection".to_string(),
                "ls -t for artifact selection".to_string(),
                "mtime/newest/latest artifact inference".to_string(),
            ],
            expected_outputs: vec![
                "reports/run_validation.v2.json".to_string(),
                "reports/GAPS.md".to_string(),
                "reports/target_operating_contract.v2.json".to_string(),
            ],
            version_policy: WorkflowVersionPolicy {
                claim_producing_run_requires_no_skew: true,
                allow_skew_override_records_gap: true,
                skew_blocks_fullset_measured_claims: true,
            },
            evidence_policy: WorkflowEvidencePolicy {
                recommendation_is_target_measurement_evidence: false,
                raw_primitives_are_claim_producing: false,
                causal_linkage_required: true,
                forbidden_linkage_sources: vec![
                    "filename_order".to_string(),
                    "mtime".to_string(),
                    "directory_co_presence".to_string(),
                    "static_prompt_version_choreography".to_string(),
                ],
            },
            next_step: "run adc-lab collect plan for an executable handoff contract".to_string(),
        },
        now_unix_ms(),
    );
    artifact.data_quality = DataQuality {
        level: DataQualityLevel::Complete,
        notes: vec![
            "workflow authority only".to_string(),
            "not target measurement evidence".to_string(),
        ],
    };
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recommendation_is_not_target_measurement_evidence() {
        let artifact =
            target_operating_contract_workflow_recommendation(WorkflowRecommendationInput {
                run_id: "LAB-RUN-test".to_string(),
                goal: WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET.to_string(),
                target: "ssh://target55".to_string(),
                target_id: "target55".to_string(),
                target_class: "raspberry_pi_4".to_string(),
                recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
            })
            .unwrap();

        assert_eq!(artifact.kind, Kind::WorkflowRecommendation);
        assert!(matches!(artifact.status, Status::NotApplicable { .. }));
        assert!(artifact.claims.is_empty());
        assert!(
            !artifact
                .payload
                .evidence_policy
                .recommendation_is_target_measurement_evidence
        );
        assert!(
            !artifact
                .payload
                .evidence_policy
                .raw_primitives_are_claim_producing
        );
    }
}
