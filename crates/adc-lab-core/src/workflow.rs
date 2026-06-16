use crate::build_info::build_info;
use crate::contracts::BuildInfo;
use crate::evidence::{Artifact, DataQuality, DataQualityLevel, Kind, Status};
use crate::ids::{new_id, now_unix_ms};
use crate::run_validation::GovernorValidity;
use crate::workflow_characterization::cpu_thermal_characterization_steps;
use crate::workflow_pressure::pressure_composite_characterization_steps;
use crate::workflow_profile::{resolve_workflow_profile, supported_validation_profile};
pub use crate::workflow_profile::{
    WorkflowProfileDepth, WORKFLOW_PROFILE_CHARACTERIZATION_FULL,
    WORKFLOW_PROFILE_LEGACY_FULLSET as WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET,
    WORKFLOW_PROFILE_SMOKE,
};
use crate::workflow_target_local::{
    target_local_execution_guide, WorkflowTargetLocalExecutionGuide,
};
use crate::{LabError, LabResult, TargetSpec, TargetTransport};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    pub effective_profile: String,
    pub profile_depth: WorkflowProfileDepth,
    pub profile_summary: String,
    pub claim_boundary: String,
    pub coverage: String,
    pub safety_caps: String,
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
    pub effective_profile: String,
    pub profile_depth: WorkflowProfileDepth,
    pub profile_summary: String,
    pub claim_boundary: String,
    pub coverage: String,
    pub safety_caps: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_local_execution_guide: Option<WorkflowTargetLocalExecutionGuide>,
    pub expected_final_artifacts: Vec<String>,
    pub packaging_is_target_evidence: bool,
    pub packaging_failure_blocks_handoff: bool,
}

pub struct WorkflowRecommendationInput {
    pub run_id: String,
    pub goal: String,
    pub profile_depth: Option<WorkflowProfileDepth>,
    pub target: String,
    pub target_id: String,
    pub target_class: String,
    pub recommendation_mode: WorkflowRecommendationMode,
}

pub struct WorkflowCollectPlanInput {
    pub run_id: String,
    pub goal: String,
    pub profile_depth: Option<WorkflowProfileDepth>,
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
    pub network_endpoint: Option<String>,
}

pub const COLLECT_PLAN_DEFERRED_NEXT_STEP: &str =
    "collect plan PR after it is available; stop before claim-producing full-set execution and report adc-lab version/capability mismatch";

pub fn validate_workflow_goal(goal: &str) -> LabResult<()> {
    if supported_validation_profile(goal) {
        Ok(())
    } else {
        Err(LabError::Validation(format!(
            "unsupported workflow profile {}; expected {}, {}, or legacy {} with --profile-depth",
            goal,
            WORKFLOW_PROFILE_SMOKE,
            WORKFLOW_PROFILE_CHARACTERIZATION_FULL,
            WORKFLOW_GOAL_TARGET_OPERATING_CONTRACT_FULLSET
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
    let profile = resolve_workflow_profile(&input.goal, input.profile_depth)?;
    let target_is_ssh = input.target.starts_with("ssh://");
    let mut must_use = vec![
        "adc-lab collect plan or equivalent workflow.collect_plan artifact".to_string(),
        "adc-lab control governor-sweep prepare/approve/run for governor evidence".to_string(),
        "adc-lab report validate-run before controlled-governor operating-contract claims"
            .to_string(),
    ];
    if target_is_ssh {
        must_use.push(
            "For SSH targets, set ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target when the release installer default is not on the non-interactive SSH PATH."
                .to_string(),
        );
    }
    let mut artifact = Artifact::new(
        Kind::WorkflowRecommendation,
        new_id("WORKFLOW-RECOMMENDATION"),
        input.run_id,
        input.target_id.clone(),
        Status::NotApplicable {
            reason: "workflow authority artifact; not target measurement evidence".to_string(),
        },
        WorkflowRecommendationPayload {
            goal: profile.requested_profile,
            effective_profile: profile.effective_profile,
            profile_depth: profile.depth,
            profile_summary: profile.summary.to_string(),
            claim_boundary: profile.claim_boundary.to_string(),
            coverage: profile.coverage.to_string(),
            safety_caps: profile.safety_caps.to_string(),
            workflow_id: WORKFLOW_ID_TARGET_OPERATING_CONTRACT_FULLSET_V023.to_string(),
            recommendation_mode: input.recommendation_mode,
            controller_adc_lab: build_info("adc-lab"),
            target: input.target,
            target_id: input.target_id,
            target_class: input.target_class,
            source_of_truth_chain: source_of_truth_chain(),
            must_use,
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
    let profile = resolve_workflow_profile(&input.goal, input.profile_depth)?;
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
    let target_local_workload_execution_run_dir =
        format!("adc-lab-target-local-workload-{}", input.run_id);
    let target_local_workload_inputs_dir =
        format!("{target_local_workload_execution_run_dir}/inputs");
    let retrieved_target_local_workload_run_dir = format!(
        "{}/included/target-local-workload-demand",
        input.planned_run_dir
    );
    let retrieved_target_local_parent_dir = format!("{}/included", input.planned_run_dir);
    let target_local_workload_scp_source = format!(
        "{}:{target_local_workload_execution_run_dir}",
        target_spec.endpoint
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
    let workload_run_dir = if target_is_ssh {
        target_local_workload_execution_run_dir.clone()
    } else {
        input.planned_run_dir.clone()
    };
    let workload_target = if target_is_ssh {
        "local"
    } else {
        input.target.as_str()
    };
    let workload_working_directory = if target_is_ssh {
        "target_local_repository_root"
    } else {
        "repository_root"
    };
    let workload_execution_location = if target_is_ssh {
        "target_local"
    } else {
        "controller"
    };
    let workload_requires_controller = !target_is_ssh;
    let policy_request_path =
        format!("{governor_approvals_dir}/governor_sweep_policy_request.v2.json");
    let policy_path = format!("{governor_approvals_dir}/governor_sweep_policy.v2.json");
    let validation_path = format!("{reports_dir}/run_validation.v2.json");
    let governor_validation_path = format!("{governor_reports_dir}/run_validation.v2.json");
    let gaps_path = format!("{reports_dir}/GAPS.md");
    let governor_gaps_path = format!("{governor_reports_dir}/GAPS.md");
    let contract_path = format!("{reports_dir}/target_operating_contract.v2.json");
    let controller_workload_run_plan_path =
        format!("{}/inputs/workload_run_plan.yaml", input.planned_run_dir);
    let workload_run_plan_path = if target_is_ssh {
        format!("{target_local_workload_inputs_dir}/workload_run_plan.yaml")
    } else {
        controller_workload_run_plan_path.clone()
    };
    let target_local_workload_plan_scp_dest =
        format!("{}:{workload_run_plan_path}", target_spec.endpoint);
    let workload_demand_path = if target_is_ssh {
        format!("{retrieved_target_local_workload_run_dir}/reports/workload_demand_profile.json")
    } else {
        input.workload_demand_path.clone()
    };
    let workload_step_output_path = if target_is_ssh {
        format!("{target_local_workload_execution_run_dir}/reports/workload_demand_profile.json")
    } else {
        input.workload_demand_path.clone()
    };
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
        profile.effective_profile.clone(),
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
    let include_run_args = if target_is_ssh {
        vec![
            "--include-run".to_string(),
            retrieved_target_local_run_dir.clone(),
        ]
    } else {
        Vec::new()
    };
    if target_is_ssh {
        run_validation_argv.splice(5..5, include_run_args.clone());
        run_validation_notes.push(format!(
            "copy or mount the target-local governor run into {retrieved_target_local_run_dir} before validation; directory co-presence alone is not causal evidence"
        ));
    }
    let mut operating_contract_argv = vec![
        "adc-lab".to_string(),
        "report".to_string(),
        "operating-contract".to_string(),
        "--run".to_string(),
        input.planned_run_dir.clone(),
        "--target-id".to_string(),
        input.target_id.clone(),
        "--target-class".to_string(),
        input.target_class.clone(),
        "--validation".to_string(),
        validation_path.clone(),
        "--strict-fullset".to_string(),
        "--json".to_string(),
    ];
    if target_is_ssh {
        operating_contract_argv.splice(5..5, include_run_args.clone());
    }

    let mut steps = vec![
        collect_step(
            "workflow_recommendation",
            "authority",
            vec![
                "adc-lab",
                "workflow",
                "recommend",
                "--goal",
                &input.goal,
                "--profile-depth",
                profile.depth.as_str(),
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
            vec![format!(
                "{}/inventory/target_inventory.json",
                input.planned_run_dir
            )],
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
    ];

    if profile.depth == WorkflowProfileDepth::CharacterizationFull {
        steps.extend(cpu_thermal_characterization_steps(
            &input.target,
            &input.planned_run_dir,
        ));
        steps.extend(pressure_composite_characterization_steps(
            &input.target,
            &input.planned_run_dir,
            input.network_endpoint.as_deref(),
        ));
    } else {
        steps.extend(vec![
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
                    "--artifact-label",
                    "observe_baseline",
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
                    format!("{}/observations/observe_baseline.*.v2.json", input.planned_run_dir),
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
                "Run the first bounded pressure probe; characterization-full expands pressure coverage.",
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
        ]);
    }

    steps.extend(vec![
        collect_step_at(
            "workload_demand",
            "workload",
            vec![
                "adc-lab",
                "workload",
                "run",
                "--target",
                workload_target,
                "--plan",
                &workload_run_plan_path,
                "--target-id",
                &input.target_id,
                "--run-dir",
                &workload_run_dir,
                "--execution-mode",
                "target-local",
                "--json",
            ],
            workload_working_directory,
            workload_execution_location,
            workload_requires_controller,
            target_is_ssh,
            false,
            false,
            vec!["workload"],
            vec![workload_step_output_path.clone()],
            "workload_demand_required_for_suitability",
            vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
            vec![GovernorValidity::Refused, GovernorValidity::Unknown],
            vec![
                "operator must provide the workload run plan; refused workload artifacts cannot support suitability claims".to_string(),
                if target_is_ssh {
                    format!(
                        "stage {controller_workload_run_plan_path} on the target as {workload_run_plan_path}, then retrieve the target-local workload run into {retrieved_target_local_workload_run_dir} before suitability"
                    )
                } else {
                    "local workload demand stays in the primary run directory".to_string()
                },
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
            operating_contract_argv,
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
                &workload_demand_path,
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
    ]);

    if target_is_ssh {
        let workload_insert_index = steps
            .iter()
            .position(|step| step.step_id == "workload_demand")
            .unwrap_or(steps.len());
        steps.splice(
            workload_insert_index..workload_insert_index,
            vec![
                collect_step_at(
                    "prepare_target_local_workload_plan_dir",
                    "workload",
                    vec!["mkdir", "-p", &target_local_workload_inputs_dir],
                    "target_local_repository_root",
                    "target_local",
                    false,
                    true,
                    false,
                    false,
                    Vec::<&str>::new(),
                    vec![target_local_workload_inputs_dir.clone()],
                    "workload_plan_staging_required_for_target_local_workload",
                    vec![GovernorValidity::NotApplicable],
                    vec![GovernorValidity::Refused],
                    vec![
                        "create the target-local workload input directory before scp staging"
                            .to_string(),
                        "run this argv on the target-local host; it is not a remote shell step"
                            .to_string(),
                    ],
                    "Create the target-local workload run input directory before staging the workload plan.",
                ),
                collect_step_at(
                    "stage_target_local_workload_plan",
                    "workload",
                    vec![
                        "scp",
                        &controller_workload_run_plan_path,
                        &target_local_workload_plan_scp_dest,
                    ],
                    "repository_root",
                    "operator_handoff",
                    true,
                    true,
                    false,
                    false,
                    Vec::<&str>::new(),
                    vec![workload_run_plan_path.clone()],
                    "workload_plan_staging_required_for_target_local_workload",
                    vec![GovernorValidity::NotApplicable],
                    vec![GovernorValidity::Refused],
                    vec![
                        "stage the exact controller workload run plan path to the target-local workload run path".to_string(),
                        "this staging step is handoff plumbing, not target measurement evidence"
                            .to_string(),
                    ],
                    "Stage the workload run plan onto the target before target-local workload demand collection.",
                ),
            ],
        );

        let retrieval_insert_index = steps
            .iter()
            .position(|step| step.step_id == "workload_demand")
            .map(|index| index + 1)
            .unwrap_or(steps.len());
        steps.splice(
            retrieval_insert_index..retrieval_insert_index,
            vec![
                collect_step_at(
                    "prepare_target_local_workload_retrieval_parent",
                    "workload",
                    vec!["mkdir", "-p", &retrieved_target_local_parent_dir],
                    "repository_root",
                    "operator_handoff",
                    true,
                    false,
                    false,
                    false,
                    Vec::<&str>::new(),
                    vec![retrieved_target_local_parent_dir.clone()],
                    "handoff_only_not_target_evidence",
                    vec![GovernorValidity::NotApplicable],
                    vec![GovernorValidity::Refused],
                    vec!["create the included-run parent before workload demand retrieval"
                        .to_string()],
                    "Create the included-run parent directory before retrieving target-local workload demand.",
                ),
                collect_step_at(
                    "reset_target_local_workload_retrieval_destination",
                    "workload",
                    vec!["rm", "-rf", &retrieved_target_local_workload_run_dir],
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
                    vec![
                        format!(
                            "rerun policy: delete only the deterministic retrieved workload path {retrieved_target_local_workload_run_dir} before scp"
                        ),
                        "this cleanup is controller-side handoff plumbing, not target evidence"
                            .to_string(),
                    ],
                    "Reset only the deterministic retrieved workload demand destination before scp so reruns keep the same layout.",
                ),
                collect_step_at(
                    "retrieve_target_local_workload_demand",
                    "workload",
                    vec![
                        "scp",
                        "-r",
                        &target_local_workload_scp_source,
                        &retrieved_target_local_workload_run_dir,
                    ],
                    "repository_root",
                    "operator_handoff",
                    true,
                    true,
                    false,
                    false,
                    vec!["workload"],
                    vec![workload_demand_path.clone()],
                    "workload_demand_required_for_suitability",
                    vec![GovernorValidity::Measured, GovernorValidity::Insufficient],
                    vec![GovernorValidity::Refused, GovernorValidity::Unknown],
                    vec![
                        "retrieval is a handoff step, not target measurement evidence".to_string(),
                        "decide suitability consumes the retrieved workload demand path exactly"
                            .to_string(),
                        "destination is removed before scp so existing directories cannot change the copied layout"
                            .to_string(),
                    ],
                    "Retrieve the target-local workload demand run into the deterministic included-run path before suitability.",
                ),
            ],
        );
    }

    let mut artifact = Artifact::new(
        Kind::WorkflowCollectPlan,
        new_id("WORKFLOW-COLLECT-PLAN"),
        input.run_id,
        input.target_id.clone(),
        Status::NotApplicable {
            reason: "workflow handoff artifact; not target measurement evidence".to_string(),
        },
        WorkflowCollectPlanPayload {
            goal: profile.requested_profile,
            effective_profile: profile.effective_profile,
            profile_depth: profile.depth,
            profile_summary: profile.summary.to_string(),
            claim_boundary: profile.claim_boundary.to_string(),
            coverage: profile.coverage.to_string(),
            safety_caps: profile.safety_caps.to_string(),
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
            target_local_execution_guide: target_local_execution_guide(
                target_is_ssh,
                &target_spec.endpoint,
            ),
            expected_final_artifacts: vec![
                input.workflow_recommendation_path,
                input.collect_plan_path,
                input.agent_instructions_path,
                validation_path,
                gaps_path,
                contract_path,
                workload_demand_path,
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
