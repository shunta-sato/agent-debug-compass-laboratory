use crate::build_info::build_info;
use crate::contracts::BuildInfo;
use crate::evidence::{Artifact, DataQuality, DataQualityLevel, Kind, Status};
use crate::ids::{new_id, now_unix_ms};
use crate::run_validation::GovernorValidity;
use crate::{LabError, LabResult, TargetSpec, TargetTransport};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCollectPlanStep {
    pub step_id: String,
    pub phase: String,
    pub command_argv: Vec<String>,
    pub working_directory_policy: String,
    pub execution_location: String,
    pub requires_target_local: bool,
    pub requires_controller: bool,
    pub requires_approval_policy: bool,
    pub requires_privileged_helper: bool,
    pub expected_artifact_kinds: Vec<String>,
    pub expected_artifact_paths_or_globs: Vec<String>,
    pub claim_gate: String,
    pub continue_on: Vec<GovernorValidity>,
    pub stop_on: Vec<GovernorValidity>,
    pub validation_after_step: Vec<String>,
    pub human_summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowContinuationRule {
    pub outcome: GovernorValidity,
    pub semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkflowCollectPlanPayload {
    pub goal: String,
    pub workflow_id: String,
    pub recommendation_mode: WorkflowRecommendationMode,
    pub controller_adc_lab: BuildInfo,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub workflow_recommendation_ref: Option<String>,
    pub workflow_recommendation_digest: Option<String>,
    pub planned_run_dir: String,
    pub source_of_truth_chain: Vec<String>,
    pub steps: Vec<WorkflowCollectPlanStep>,
    pub continuation_semantics: Vec<WorkflowContinuationRule>,
    pub expected_final_artifacts: Vec<String>,
    pub packaging_is_target_evidence: bool,
    pub packaging_failure_blocks_handoff: bool,
}

pub struct WorkflowRecommendationInput {
    pub run_id: String,
    pub goal: String,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub recommendation_mode: WorkflowRecommendationMode,
}

pub struct WorkflowCollectPlanInput {
    pub run_id: String,
    pub goal: String,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub planned_run_dir: String,
    pub collect_plan_path: String,
    pub agent_instructions_path: String,
    pub handoff_dir: String,
    pub workflow_recommendation_path: String,
    pub workflow_recommendation_ref: Option<String>,
    pub workflow_recommendation_digest: Option<String>,
    pub workload_demand_path: String,
    pub suitability_policy_path: String,
    pub expected_governors: Vec<String>,
    pub recommendation_mode: WorkflowRecommendationMode,
}

pub const COLLECT_PLAN_DEFERRED_NEXT_STEP: &str =
    "collect plan PR after it is available; stop before claim-producing full-set execution and report adc-lab version/capability mismatch";

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

fn source_of_truth_chain() -> Vec<String> {
    vec![
        "workflow.recommendation".to_string(),
        "workflow.collect_plan".to_string(),
        "report.run_validation".to_string(),
        "report.operating_contract".to_string(),
        "report.suitability".to_string(),
        "report.constraints".to_string(),
    ]
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
            source_of_truth_chain: source_of_truth_chain(),
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

pub fn target_operating_contract_collect_plan(
    input: WorkflowCollectPlanInput,
) -> LabResult<Artifact<WorkflowCollectPlanPayload>> {
    validate_workflow_goal(&input.goal)?;
    let target_spec = TargetSpec::parse(&input.target)?;
    let target_is_ssh = matches!(target_spec.transport, TargetTransport::Ssh);
    let governors = if input.expected_governors.is_empty() {
        default_fullset_governors()
    } else {
        input.expected_governors
    };
    let governor_arg = governors.join(",");
    let reports_dir = format!("{}/reports", input.planned_run_dir);
    let target_local_execution_run_dir = format!("adc-lab-target-local-{}", input.run_id);
    let retrieved_target_local_run_dir = format!(
        "{}/included/target-local-governor-sweep",
        input.planned_run_dir
    );
    let governor_run_dir = if target_is_ssh {
        target_local_execution_run_dir.clone()
    } else {
        input.planned_run_dir.clone()
    };
    let governor_reports_dir = format!("{governor_run_dir}/reports");
    let governor_approvals_dir = format!("{governor_run_dir}/approvals");
    let governor_target = if target_is_ssh {
        "local"
    } else {
        input.target.as_str()
    };
    let governor_working_directory = if target_is_ssh {
        "target_local_repository_root"
    } else {
        "repository_root"
    };
    let governor_execution_location = if target_is_ssh {
        "target_local"
    } else {
        "controller"
    };
    let governor_requires_controller = !target_is_ssh;
    let policy_request_path =
        format!("{governor_approvals_dir}/governor_sweep_policy_request.v2.json");
    let policy_path = format!("{governor_approvals_dir}/governor_sweep_policy.v2.json");
    let validation_path = format!("{reports_dir}/run_validation.v2.json");
    let governor_validation_path = format!("{governor_reports_dir}/run_validation.v2.json");
    let gaps_path = format!("{reports_dir}/GAPS.md");
    let governor_gaps_path = format!("{governor_reports_dir}/GAPS.md");
    let contract_path = format!("{reports_dir}/target_operating_contract.v2.json");
    let workload_run_plan_path = format!("{}/inputs/workload_run_plan.yaml", input.planned_run_dir);
    let suitability_path = format!("{reports_dir}/suitability.v2.json");
    let constraints_path = format!("{reports_dir}/constraints.v2.json");
    let constraints_markdown_path = format!("{reports_dir}/agent_constraints.md");
    let archive_path = format!("{}/{}.tgz", input.handoff_dir, input.run_id);
    let mut run_validation_argv = vec![
        "adc-lab".to_string(),
        "report".to_string(),
        "validate-run".to_string(),
        "--run".to_string(),
        input.planned_run_dir.clone(),
        "--profile".to_string(),
        input.goal.clone(),
        "--expected-governors".to_string(),
        governor_arg.clone(),
        "--workflow-recommendation".to_string(),
        input.workflow_recommendation_path.clone(),
        "--collect-plan".to_string(),
        input.collect_plan_path.clone(),
        "--target-id".to_string(),
        input.target_id.clone(),
        "--target-class".to_string(),
        input.target_class.clone(),
        "--out".to_string(),
        validation_path.clone(),
        "--gaps-out".to_string(),
        gaps_path.clone(),
        "--allow-non-measured".to_string(),
        "--json".to_string(),
    ];
    let mut run_validation_notes =
        vec!["selection_ready remains false unless overall validity is measured".to_string()];
    if target_is_ssh {
        run_validation_argv.insert(5, retrieved_target_local_run_dir.clone());
        run_validation_argv.insert(5, "--include-run".to_string());
        run_validation_notes.push(format!(
            "copy or mount the target-local governor run into {retrieved_target_local_run_dir} before validation; directory co-presence alone is not causal evidence"
        ));
    }

    let steps = vec![
        collect_step(
            "workflow_recommendation",
            "authority",
            vec![
                "adc-lab",
                "workflow",
                "recommend",
                "--goal",
                &input.goal,
                "--target",
                &input.target,
                "--target-id",
                &input.target_id,
                "--target-class",
                &input.target_class,
                "--out",
                &input.workflow_recommendation_path,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["workflow.recommendation"],
            vec![input.workflow_recommendation_path.clone()],
            "authority_only_not_measurement",
            vec![GovernorValidity::NotApplicable],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec![],
            "Create the workflow authority artifact for this collection.",
        ),
        collect_step(
            "capability_check",
            "capability",
            vec![
                "adc-lab",
                "privilege",
                "doctor",
                "--target",
                &input.target,
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            true,
            false,
            false,
            vec!["lab.privilege_doctor.v1"],
            vec![format!(
                "{}/privilege/privilege_doctor.json",
                input.planned_run_dir
            )],
            "preflight_only",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec!["review helper readiness before privileged sweep".to_string()],
            "Check target-local helper readiness before any privileged sweep.",
        ),
        collect_step(
            "read_only_inventory",
            "read_only",
            vec![
                "adc-lab",
                "inventory",
                "--target",
                &input.target,
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["lab.target_inventory.v1"],
            vec![format!("{}/inventory/target_inventory.json", input.planned_run_dir)],
            "read_only_inventory_required",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec!["target identity and hardware claims require this read-only artifact".to_string()],
            "Collect read-only target inventory before claim-producing reports.",
        ),
        collect_step(
            "toolchain_discover",
            "read_only",
            vec![
                "adc-lab",
                "toolchain",
                "discover",
                "--target",
                &input.target,
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["lab.toolchain_inventory.v1"],
            vec![format!(
                "{}/toolchain/toolchain_inventory.json",
                input.planned_run_dir
            )],
            "read_only_toolchain_required",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec!["tool availability claims require this read-only artifact".to_string()],
            "Collect read-only toolchain inventory before workload or pressure claims.",
        ),
        collect_step(
            "observe_baseline",
            "read_only",
            vec![
                "adc-lab",
                "observe",
                "--target",
                &input.target,
                "--duration",
                "30s",
                "--sample-interval",
                "1s",
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["observation"],
            vec![
                format!("{}/observations/observe.json", input.planned_run_dir),
                format!("{}/observations/observe.v2.json", input.planned_run_dir),
            ],
            "baseline_observation_required",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec!["baseline observation is context, not a controlled sweep".to_string()],
            "Collect passive baseline observation before pressure or suitability reports.",
        ),
        collect_step(
            "cpu_ladder",
            "load",
            vec![
                "adc-lab",
                "load",
                "cpu",
                "--target",
                &input.target,
                "--workers",
                "1",
                "--duration",
                "10s",
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["load"],
            Vec::<String>::new(),
            "bounded_load_seed_not_full_ladder",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec![
                "this bounded load seed does not prove all CPU ladder points; expand only through typed plan revisions".to_string(),
            ],
            "Run a bounded CPU load seed so full-set coverage includes load evidence.",
        ),
        collect_step(
            "pressure_probe_set",
            "pressure",
            vec![
                "adc-lab",
                "pressure",
                "run",
                "--target",
                &input.target,
                "--kind",
                "memory_pressure",
                "--duration",
                "5s",
                "--workers",
                "1",
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["pressure"],
            Vec::<String>::new(),
            "pressure_probe_required_for_pressure_claims",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec!["run additional pressure kinds only through typed plan revisions".to_string()],
            "Run the first bounded pressure probe in the full-set coverage map.",
        ),
        collect_step(
            "composite_probe",
            "pressure",
            vec![
                "adc-lab",
                "pressure",
                "composite",
                "--target",
                &input.target,
                "--scenario",
                "memory_storage_jitter",
                "--duration",
                "5s",
                "--workers",
                "1",
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["composite"],
            Vec::<String>::new(),
            "composite_probe_required_for_coupling_claims",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec!["composite status must be measured before coupling claims are supported".to_string()],
            "Run the bounded composite pressure probe for coupling coverage.",
        ),
        collect_step(
            "workload_demand",
            "workload",
            vec![
                "adc-lab",
                "workload",
                "run",
                "--target",
                &input.target,
                "--plan",
                &workload_run_plan_path,
                "--target-id",
                &input.target_id,
                "--run-dir",
                &input.planned_run_dir,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["workload"],
            vec![input.workload_demand_path.clone()],
            "workload_demand_required_for_suitability",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec![
                "operator must provide the workload run plan; refused workload artifacts cannot support suitability claims".to_string(),
            ],
            "Generate or preserve workload demand evidence using an explicit workload plan path.",
        ),
        collect_step_at(
            "governor_sweep_prepare",
            "approval",
            vec![
                "adc-lab",
                "control",
                "governor-sweep",
                "prepare",
                "--target",
                governor_target,
                "--governors",
                &governor_arg,
                "--duration-seconds-max",
                "60",
                "--run-dir",
                &governor_run_dir,
                "--out",
                &policy_request_path,
                "--json",
            ],
            governor_working_directory,
            governor_execution_location,
            governor_requires_controller,
            true,
            false,
            false,
            vec!["control.governor_sweep_policy"],
            vec![policy_request_path.clone()],
            "human_approval_required_before_apply",
            vec![GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec![
                "human reviews requested governors, target, duration, and expiry".to_string(),
                "ssh controller workflows execute governor sweep argv on the target-local adc-lab with --target local".to_string(),
            ],
            "Prepare the bounded governor sweep policy request.",
        ),
        collect_step_at(
            "governor_sweep_approve",
            "approval",
            vec![
                "adc-lab",
                "control",
                "governor-sweep",
                "approve",
                "--request",
                &policy_request_path,
                "--approved-by",
                "operator",
                "--run-dir",
                &governor_run_dir,
                "--out",
                &policy_path,
                "--json",
            ],
            governor_working_directory,
            governor_execution_location,
            governor_requires_controller,
            true,
            true,
            false,
            vec!["control.governor_sweep_policy"],
            vec![policy_path.clone()],
            "human_approval_required",
            vec![GovernorValidity::Measured],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec!["approved policy digest must match the request".to_string()],
            "Record out-of-band human approval bound to the sweep policy digest.",
        ),
        collect_step_at(
            "governor_sweep_run",
            "control",
            vec![
                "adc-lab",
                "control",
                "governor-sweep",
                "run",
                "--target",
                governor_target,
                "--governors",
                &governor_arg,
                "--approval-policy",
                &policy_path,
                "--duration-seconds-max",
                "60",
                "--load-duration",
                "1s",
                "--restore-after-each",
                "--run-dir",
                &governor_run_dir,
                "--allow-non-measured",
                "--json",
            ],
            governor_working_directory,
            governor_execution_location,
            governor_requires_controller,
            true,
            true,
            true,
            vec![
                "report.run_validation",
                "control_result",
                "load",
                "lab.restore_lease.v1",
            ],
            vec![governor_validation_path.clone(), governor_gaps_path.clone()],
            "claim_gate_requires_report_run_validation",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec![
                "non-measured sweep evidence is preserved but cannot support full-set claims"
                    .to_string(),
                "for ssh controller workflows, retrieve the target-local run directory before controller-side validation".to_string(),
            ],
            "Run the approved bounded governor sweep and preserve validation gaps.",
        ),
        collect_step(
            "run_validation",
            "validation",
            run_validation_argv,
            "repository_root",
            true,
            false,
            false,
            vec!["report.run_validation"],
            vec![validation_path.clone(), gaps_path.clone()],
            "required_fullset_validation",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![
                GovernorValidity::Refused,
                GovernorValidity::Contaminated,
                GovernorValidity::Unknown,
            ],
            run_validation_notes,
            "Validate the run set using typed collect-plan and recommendation refs.",
        ),
        collect_step(
            "operating_contract",
            "reporting",
            vec![
                "adc-lab",
                "report",
                "operating-contract",
                "--run",
                &input.planned_run_dir,
                "--target-id",
                &input.target_id,
                "--target-class",
                &input.target_class,
                "--validation",
                &validation_path,
                "--strict-fullset",
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["report.operating_contract"],
            vec![contract_path.clone()],
            "downstream_claims_require_matching_run_validation",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Contaminated],
            vec!["Phase 5 will add strict operating-contract validation gate".to_string()],
            "Generate the operating contract from validated evidence available in the run.",
        ),
        collect_step(
            "suitability",
            "suitability",
            vec![
                "adc-lab",
                "decide",
                "suitability",
                "--target-run",
                &input.planned_run_dir,
                "--target-contract",
                &contract_path,
                "--workload-demand",
                &input.workload_demand_path,
                "--policy",
                &input.suitability_policy_path,
                "--out",
                &suitability_path,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["report.suitability"],
            vec![suitability_path.clone()],
            "operator_provided_inputs_required",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec![
                "operator must provide workload demand JSON and suitability policy YAML"
                    .to_string(),
            ],
            "Evaluate suitability using explicit input paths, not discovered files.",
        ),
        collect_step(
            "constraints",
            "constraints",
            vec![
                "adc-lab",
                "constraints",
                "generate",
                "--decision",
                &suitability_path,
                "--out",
                &constraints_path,
                "--agent-instructions-out",
                &constraints_markdown_path,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["report.constraints"],
            vec![constraints_path.clone(), constraints_markdown_path.clone()],
            "generated_constraints_are_negative_or_explanatory",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec![
                "run constraints self-check against the generated markdown before handoff"
                    .to_string(),
            ],
            "Generate Agent-facing constraints from the suitability artifact.",
        ),
        collect_step(
            "constraints_self_check",
            "constraints",
            vec![
                "adc-lab",
                "constraints",
                "self-check",
                "--constraints",
                &constraints_path,
                "--path",
                &constraints_markdown_path,
                "--json",
            ],
            "repository_root",
            false,
            false,
            false,
            vec!["report.constraints_check"],
            Vec::<String>::new(),
            "generated_constraints_are_negative_or_explanatory",
            vec![GovernorValidity::Measured, GovernorValidity::MeasuredPartial],
            vec![
                GovernorValidity::Insufficient,
                GovernorValidity::Refused,
                GovernorValidity::Contaminated,
                GovernorValidity::Unknown,
            ],
            vec!["self-check must pass before handing generated constraints to an Agent".to_string()],
            "Verify generated constraints before handoff; blocked claims are allowed only in generated negative or explanatory sections.",
        ),
        collect_step_at(
            "handoff_prepare",
            "handoff",
            vec!["mkdir", "-p", &input.handoff_dir],
            "repository_root",
            "operator_handoff",
            true,
            false,
            false,
            false,
            Vec::<&str>::new(),
            Vec::<String>::new(),
            "handoff_only_not_target_evidence",
            vec![GovernorValidity::NotApplicable],
            vec![GovernorValidity::Refused],
            vec!["create archive output directory outside the run directory".to_string()],
            "Create the handoff directory outside the run directory before packaging.",
        ),
        collect_step_at(
            "archive",
            "handoff",
            vec![
                "tar",
                "-czf",
                &archive_path,
                "-C",
                &input.planned_run_dir,
                ".",
            ],
            "repository_root",
            "operator_handoff",
            true,
            false,
            false,
            false,
            Vec::<&str>::new(),
            vec![archive_path.clone()],
            "handoff_only_not_target_evidence",
            vec![
                GovernorValidity::Measured,
                GovernorValidity::Insufficient,
                GovernorValidity::NotApplicable,
            ],
            vec![GovernorValidity::Refused],
            vec![
                "packaging failure blocks handoff completion but not measurement validity"
                    .to_string(),
            ],
            "Package the run directory for handoff; this is not measurement evidence.",
        ),
        collect_step_at(
            "checksum",
            "handoff",
            vec!["sha256sum", &archive_path],
            "repository_root",
            "operator_handoff",
            true,
            false,
            false,
            false,
            Vec::<&str>::new(),
            Vec::<String>::new(),
            "handoff_only_not_target_evidence",
            vec![
                GovernorValidity::Measured,
                GovernorValidity::Insufficient,
                GovernorValidity::NotApplicable,
            ],
            vec![GovernorValidity::Refused],
            vec!["record checksum stdout in the handoff notes".to_string()],
            "Create a checksum for the handoff archive; stdout is the handoff record.",
        ),
    ];

    let mut artifact = Artifact::new(
        Kind::WorkflowCollectPlan,
        new_id("WORKFLOW-COLLECT-PLAN"),
        input.run_id,
        input.target_id.clone(),
        Status::NotApplicable {
            reason: "workflow handoff artifact; not target measurement evidence".to_string(),
        },
        WorkflowCollectPlanPayload {
            goal: input.goal,
            workflow_id: WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023.to_string(),
            recommendation_mode: input.recommendation_mode,
            controller_adc_lab: build_info("adc-lab"),
            target: input.target,
            target_id: input.target_id,
            target_class: input.target_class,
            workflow_recommendation_ref: input.workflow_recommendation_ref,
            workflow_recommendation_digest: input.workflow_recommendation_digest,
            planned_run_dir: input.planned_run_dir,
            source_of_truth_chain: source_of_truth_chain(),
            steps,
            continuation_semantics: continuation_semantics(),
            expected_final_artifacts: vec![
                input.workflow_recommendation_path,
                input.collect_plan_path,
                input.agent_instructions_path,
                validation_path,
                gaps_path,
                contract_path,
                input.workload_demand_path,
                suitability_path,
                constraints_path,
                constraints_markdown_path,
                archive_path,
            ],
            packaging_is_target_evidence: false,
            packaging_failure_blocks_handoff: true,
        },
        now_unix_ms(),
    );
    artifact.data_quality = DataQuality {
        level: DataQualityLevel::Complete,
        notes: vec![
            "workflow handoff only".to_string(),
            "commands are argv arrays, not shell fragments".to_string(),
            "packaging steps are not target evidence".to_string(),
        ],
    };
    Ok(artifact)
}

pub fn render_collect_plan_agent_instructions(
    collect_plan: &Artifact<WorkflowCollectPlanPayload>,
) -> String {
    let payload = &collect_plan.payload;
    let mut out = String::new();
    out.push_str("# adc-lab Collect Plan Instructions\n\n");
    out.push_str("Use this workflow.collect_plan artifact as the executable handoff contract.\n\n");
    out.push_str(&format!("- goal: `{}`\n", payload.goal));
    out.push_str(&format!("- workflow_id: `{}`\n", payload.workflow_id));
    out.push_str(&format!("- target: `{}`\n", payload.target));
    out.push_str(&format!("- target_id: `{}`\n", payload.target_id));
    out.push_str(&format!("- target_class: `{}`\n", payload.target_class));
    out.push_str(&format!(
        "- planned_run_dir: `{}`\n\n",
        payload.planned_run_dir
    ));
    out.push_str("Do not fall back to a static prompt or hand-written shell harness when this collect plan is available.\n");
    out.push_str("If a required workflow surface is missing, stop and report adc-lab version/capability mismatch.\n");
    out.push_str("Do not infer artifact relationships from path names, timestamps, or directory co-presence.\n\n");
    out.push_str("## Steps\n\n");
    for step in &payload.steps {
        out.push_str(&format!("### `{}`\n\n", step.step_id));
        out.push_str(&format!("- phase: `{}`\n", step.phase));
        out.push_str(&format!(
            "- execution_location: `{}`\n",
            step.execution_location
        ));
        out.push_str(&format!("- claim_gate: `{}`\n", step.claim_gate));
        out.push_str(&format!(
            "- argv: `{}`\n",
            serde_json::to_string(&step.command_argv).unwrap_or_else(|_| "[]".to_string())
        ));
        if !step.expected_artifact_paths_or_globs.is_empty() {
            out.push_str("- expected_paths:\n");
            for path in &step.expected_artifact_paths_or_globs {
                out.push_str(&format!("  - `{path}`\n"));
            }
        }
        out.push_str(&format!("- summary: {}\n\n", step.human_summary));
    }
    out.push_str("Packaging steps are handoff steps, not target evidence. Packaging failure blocks final handoff completion but does not change measurement validity.\n");
    out
}

fn default_fullset_governors() -> Vec<String> {
    vec![
        "ondemand".to_string(),
        "performance".to_string(),
        "powersave".to_string(),
    ]
}

#[allow(clippy::too_many_arguments)]
fn collect_step<S, P>(
    step_id: &str,
    phase: &str,
    command_argv: Vec<S>,
    working_directory_policy: &str,
    requires_target_local: bool,
    requires_approval_policy: bool,
    requires_privileged_helper: bool,
    expected_artifact_kinds: Vec<&str>,
    expected_artifact_paths_or_globs: Vec<P>,
    claim_gate: &str,
    continue_on: Vec<GovernorValidity>,
    stop_on: Vec<GovernorValidity>,
    validation_after_step: Vec<String>,
    human_summary: &str,
) -> WorkflowCollectPlanStep
where
    S: Into<String>,
    P: Into<String>,
{
    collect_step_at(
        step_id,
        phase,
        command_argv,
        working_directory_policy,
        "controller",
        true,
        requires_target_local,
        requires_approval_policy,
        requires_privileged_helper,
        expected_artifact_kinds,
        expected_artifact_paths_or_globs,
        claim_gate,
        continue_on,
        stop_on,
        validation_after_step,
        human_summary,
    )
}

#[allow(clippy::too_many_arguments)]
fn collect_step_at<S, P>(
    step_id: &str,
    phase: &str,
    command_argv: Vec<S>,
    working_directory_policy: &str,
    execution_location: &str,
    requires_controller: bool,
    requires_target_local: bool,
    requires_approval_policy: bool,
    requires_privileged_helper: bool,
    expected_artifact_kinds: Vec<&str>,
    expected_artifact_paths_or_globs: Vec<P>,
    claim_gate: &str,
    continue_on: Vec<GovernorValidity>,
    stop_on: Vec<GovernorValidity>,
    validation_after_step: Vec<String>,
    human_summary: &str,
) -> WorkflowCollectPlanStep
where
    S: Into<String>,
    P: Into<String>,
{
    WorkflowCollectPlanStep {
        step_id: step_id.to_string(),
        phase: phase.to_string(),
        command_argv: command_argv.into_iter().map(Into::into).collect(),
        working_directory_policy: working_directory_policy.to_string(),
        execution_location: execution_location.to_string(),
        requires_target_local,
        requires_controller,
        requires_approval_policy,
        requires_privileged_helper,
        expected_artifact_kinds: expected_artifact_kinds
            .into_iter()
            .map(str::to_string)
            .collect(),
        expected_artifact_paths_or_globs: expected_artifact_paths_or_globs
            .into_iter()
            .map(Into::into)
            .collect(),
        claim_gate: claim_gate.to_string(),
        continue_on,
        stop_on,
        validation_after_step,
        human_summary: human_summary.to_string(),
    }
}

fn continuation_semantics() -> Vec<WorkflowContinuationRule> {
    vec![
        WorkflowContinuationRule {
            outcome: GovernorValidity::Insufficient,
            semantics: "preserve evidence; continue unless this step is a required validation gate"
                .to_string(),
        },
        WorkflowContinuationRule {
            outcome: GovernorValidity::Refused,
            semantics: "preserve refusal; continue only when exploratory continuation is explicit"
                .to_string(),
        },
        WorkflowContinuationRule {
            outcome: GovernorValidity::Contaminated,
            semantics: "preserve evidence; do not use for claim-producing downstream steps"
                .to_string(),
        },
        WorkflowContinuationRule {
            outcome: GovernorValidity::Unknown,
            semantics:
                "preserve gap; downstream selection_ready remains false for required dimensions"
                    .to_string(),
        },
        WorkflowContinuationRule {
            outcome: GovernorValidity::NotApplicable,
            semantics: "preserve boundary and continue only if the claim gate does not require it"
                .to_string(),
        },
    ]
}

pub fn render_codex_agent_instructions(
    recommendation: &Artifact<WorkflowRecommendationPayload>,
    collect_plan_available: bool,
) -> String {
    let payload = &recommendation.payload;
    let next_step = if collect_plan_available {
        "run adc-lab collect plan, then follow the emitted argv-array steps"
    } else {
        COLLECT_PLAN_DEFERRED_NEXT_STEP
    };
    let mut out = String::new();
    out.push_str("# adc-lab Controller Agent Instructions\n\n");
    out.push_str("## Workflow Authority\n\n");
    out.push_str("Use adc-lab workflow outputs as the source of truth for Target Operating Contract full-set collection.\n\n");
    out.push_str(&format!("- goal: `{}`\n", payload.goal));
    out.push_str(&format!("- workflow_id: `{}`\n", payload.workflow_id));
    out.push_str(&format!(
        "- adc_lab_version: `{}`\n",
        payload.controller_adc_lab.version
    ));
    out.push_str(&format!(
        "- adc_lab_git_sha: `{}`\n",
        payload.controller_adc_lab.git_sha
    ));
    out.push_str(&format!("- target: `{}`\n", payload.target));
    out.push_str(&format!("- target_id: `{}`\n", payload.target_id));
    out.push_str(&format!("- target_class: `{}`\n\n", payload.target_class));

    out.push_str("## Source Of Truth Chain\n\n");
    for item in &payload.source_of_truth_chain {
        out.push_str(&format!("- `{item}`\n"));
    }
    out.push('\n');

    out.push_str("## Required Workflow\n\n");
    for item in &payload.must_use {
        out.push_str(&format!("- {item}\n"));
    }
    out.push_str(&format!("- next_step: {next_step}\n\n"));

    out.push_str("## Forbidden Fallbacks\n\n");
    out.push_str("- Do not fall back to a static prompt or hand-written shell harness when adc-lab workflow surfaces are available.\n");
    out.push_str("- If a required workflow surface is missing, stop and report adc-lab version/capability mismatch.\n");
    out.push_str("- Do not infer artifact relationships from filenames, timestamps, or directory co-presence.\n");
    out.push_str("- Do not use raw primitive control artifacts for controlled-governor full-set claims without matching report.run_validation.\n\n");

    out.push_str("## Expected Outputs\n\n");
    for item in &payload.expected_outputs {
        out.push_str(&format!("- `{item}`\n"));
    }
    out.push('\n');

    out.push_str("## Claim Boundaries\n\n");
    out.push_str("- This prompt is not target measurement evidence.\n");
    out.push_str("- Insufficient, refused, contaminated, not applicable, and unknown outcomes remain valid evidence boundaries.\n");
    out.push_str("- Version skew blocks full-set measured claims unless a later validation artifact explicitly records the allowed exploratory override, and even then full-set selection remains not ready.\n");
    out.push_str("- No Agent root shell, arbitrary sysfs writes, remote privileged apply/restore, Pi4/Pi5 selection claim, or production-style claim is authorized by this prompt.\n");
    out
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

    #[test]
    fn codex_agent_instructions_are_registry_derived_without_artifact_selection_heuristics() {
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
        let text = render_codex_agent_instructions(&artifact, false);

        assert!(text.contains(WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023));
        assert!(text.contains("Do not fall back to a static prompt"));
        assert!(text.contains("stop and report adc-lab version/capability mismatch"));
        assert!(text.contains(COLLECT_PLAN_DEFERRED_NEXT_STEP));
        for forbidden in [
            "PLAN-*.json",
            "APPROVAL-*.json",
            "LEASE-*.json",
            "tail -n 1",
            "ls -t",
            "find ",
            "mtime",
            "newest",
            "latest plan",
            "latest approval",
            "latest lease",
        ] {
            assert!(
                !text.contains(forbidden),
                "generated prompt must not contain {forbidden}"
            );
        }
    }

    #[test]
    fn collect_plan_steps_are_argv_arrays_and_not_measurement_evidence() {
        let artifact = target_operating_contract_collect_plan(WorkflowCollectPlanInput {
            run_id: "LAB-RUN-test".to_string(),
            goal: WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET.to_string(),
            target: "ssh://target55".to_string(),
            target_id: "target55".to_string(),
            target_class: "raspberry_pi_4".to_string(),
            planned_run_dir: "/tmp/adc-lab-run".to_string(),
            collect_plan_path: "/tmp/adc-lab-run/workflows/collect_plan.v2.json".to_string(),
            agent_instructions_path: "/tmp/adc-lab-run/workflows/collect_plan.md".to_string(),
            handoff_dir: "/tmp/handoff".to_string(),
            workflow_recommendation_path: "/tmp/adc-lab-run/workflows/recommendation.v2.json"
                .to_string(),
            workflow_recommendation_ref: None,
            workflow_recommendation_digest: None,
            workload_demand_path: "/tmp/adc-lab-run/inputs/workload_demand.json".to_string(),
            suitability_policy_path: "/tmp/adc-lab-run/inputs/suitability_policy.yaml".to_string(),
            expected_governors: vec!["ondemand".to_string(), "performance".to_string()],
            recommendation_mode: WorkflowRecommendationMode::OfflineRecommendation,
        })
        .unwrap();

        assert_eq!(artifact.kind, Kind::WorkflowCollectPlan);
        assert!(matches!(artifact.status, Status::NotApplicable { .. }));
        assert!(artifact.claims.is_empty());
        assert!(!artifact.payload.packaging_is_target_evidence);
        assert!(artifact.payload.packaging_failure_blocks_handoff);
        assert!(artifact
            .payload
            .source_of_truth_chain
            .contains(&"report.run_validation".to_string()));

        let validation_step = artifact
            .payload
            .steps
            .iter()
            .find(|step| step.step_id == "run_validation")
            .unwrap();
        assert!(validation_step
            .command_argv
            .contains(&"--collect-plan".to_string()));
        assert!(validation_step
            .command_argv
            .contains(&"--include-run".to_string()));
        assert!(validation_step
            .expected_artifact_kinds
            .contains(&"report.run_validation".to_string()));

        let governor_step = artifact
            .payload
            .steps
            .iter()
            .find(|step| step.step_id == "governor_sweep_run")
            .unwrap();
        assert_eq!(governor_step.execution_location, "target_local");
        assert!(!governor_step.requires_controller);
        let target_arg_index = governor_step
            .command_argv
            .iter()
            .position(|arg| arg == "--target")
            .unwrap()
            + 1;
        assert_eq!(governor_step.command_argv[target_arg_index], "local");
        assert!(!governor_step
            .command_argv
            .iter()
            .any(|arg| arg == "ssh://target55"));

        let archive_step = artifact
            .payload
            .steps
            .iter()
            .find(|step| step.step_id == "archive")
            .unwrap();
        assert!(archive_step
            .command_argv
            .contains(&"/tmp/handoff/LAB-RUN-test.tgz".to_string()));
        assert!(!archive_step
            .command_argv
            .contains(&"/tmp/adc-lab-run/handoff/adc-lab-run.tgz".to_string()));

        for required_step in [
            "read_only_inventory",
            "toolchain_discover",
            "observe_baseline",
            "cpu_ladder",
            "pressure_probe_set",
            "composite_probe",
            "workload_demand",
        ] {
            assert!(
                artifact
                    .payload
                    .steps
                    .iter()
                    .any(|step| step.step_id == required_step),
                "collect plan missing full-set skeleton step {required_step}"
            );
        }

        for step in &artifact.payload.steps {
            assert!(!step.command_argv.is_empty());
            for arg in &step.command_argv {
                for forbidden in ["|", "&&", "$(", "`"] {
                    assert!(
                        !arg.contains(forbidden),
                        "argv item must not contain shell fragment {forbidden}: {arg}"
                    );
                }
            }
        }

        let instructions = render_collect_plan_agent_instructions(&artifact);
        assert!(instructions.contains("argv: `["));
        assert!(instructions.contains("Do not fall back to a static prompt"));
        for forbidden in [
            "PLAN-*.json",
            "APPROVAL-*.json",
            "LEASE-*.json",
            "tail -n 1",
            "ls -t",
            "find ",
            "mtime",
            "newest",
            "latest plan",
            "latest approval",
            "latest lease",
        ] {
            assert!(
                !instructions.contains(forbidden),
                "generated collect instructions must not contain {forbidden}"
            );
        }
    }
}
