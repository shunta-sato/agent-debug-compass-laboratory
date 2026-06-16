use crate::contracts::{
    SuitabilityConfidence, SuitabilityDecisionValue, SuitabilityDimensionDecision,
    SuitabilityDimensionKind, SuitabilityPolicy, WorkloadDemandProfile,
};
use crate::evidence::{
    claim, claim_definition, Artifact, Claim, DataQuality, DataQualityLevel, Decision, Kind, Status,
};
use crate::fsutil::read_json;
use crate::ids::{new_id, now_unix_ms};
use crate::rules::engine::{claim_for_evaluation, RuleEvaluation};
use crate::rules::suitability::SuitabilityPayload;
use crate::OperatingContractPayload;
use crate::{LabError, LabResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct TargetRunNumericEvidence {
    pub max_temp_c: Option<f64>,
    pub memory_available_min_kb: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub pressure: Vec<TargetRunPressureEvidence>,
    pub missing: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetRunPressureEvidence {
    pub kind: String,
    pub status_state: String,
    pub evidence_class: String,
    pub effect_observed: bool,
    pub network_mode: Option<String>,
    pub network_endpoint_available: Option<bool>,
    pub network_traffic_generated_bytes: Option<u64>,
    pub artifact_ref: String,
}

impl TargetRunPressureEvidence {
    fn is_measuredish(&self) -> bool {
        matches!(self.status_state.as_str(), "measured" | "measured_partial")
    }

    fn is_endpoint_backed_bounded_transfer(&self) -> bool {
        self.kind == "network_io"
            && self.is_measuredish()
            && self.effect_observed
            && self.network_mode.as_deref() == Some("bounded_transfer")
            && self.network_endpoint_available == Some(true)
            && self.network_traffic_generated_bytes.unwrap_or(0) > 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintCheckPayload {
    pub mode: ConstraintCheckMode,
    pub status: String,
    pub checked_path: String,
    pub matches: Vec<ConstraintCheckMatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintCheckMode {
    CandidateContent,
    GeneratedConstraints,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintCheckMatch {
    pub path: String,
    pub claim_id: String,
    pub blocked_claim: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstraintsPayload {
    pub source_suitability_id: String,
    pub workload_id: Option<String>,
    pub policy_id: Option<String>,
    pub allowed_patterns: Vec<String>,
    pub burst_only_patterns: Vec<String>,
    pub degraded_mode_triggers: Vec<String>,
    pub forbidden_patterns: Vec<String>,
    pub budget_constraints: Vec<String>,
    pub required_runtime_guards: Vec<String>,
    pub blocked_claims: Vec<String>,
    pub agent_instructions: Vec<String>,
    pub ci_rules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuitabilityArtifactContext {
    pub target_contract_ref: String,
    pub workload_ref: String,
    pub policy_ref: String,
    pub run_id: String,
}

pub fn decide_suitability_artifact_v2(
    target_run_dir: &Path,
    target_contract: &Artifact<OperatingContractPayload>,
    workload: &WorkloadDemandProfile,
    policy: &SuitabilityPolicy,
    context: SuitabilityArtifactContext,
) -> LabResult<Artifact<SuitabilityPayload>> {
    validate_policy(policy)?;
    let target_evidence = collect_target_run_numeric_evidence(target_run_dir);
    let mut dimensions = Vec::new();
    let mut seen = BTreeSet::new();
    for dimension in policy
        .required_dimensions
        .iter()
        .chain(policy.optional_dimensions.iter())
    {
        if seen.insert(dimension.clone()) {
            dimensions.push(decide_dimension(
                dimension.clone(),
                policy,
                workload,
                &target_evidence,
                vec![
                    context.workload_ref.clone(),
                    context.target_contract_ref.clone(),
                ],
            ));
        }
    }
    let required = dimensions
        .iter()
        .filter(|dimension| policy.required_dimensions.contains(&dimension.dimension))
        .collect::<Vec<_>>();
    let overall_decision = if required
        .iter()
        .any(|dimension| dimension.decision == SuitabilityDecisionValue::Fail)
    {
        SuitabilityDecisionValue::Fail
    } else if required
        .iter()
        .any(|dimension| dimension.decision == SuitabilityDecisionValue::Unknown)
    {
        SuitabilityDecisionValue::Unknown
    } else if required
        .iter()
        .any(|dimension| dimension.decision == SuitabilityDecisionValue::Marginal)
    {
        SuitabilityDecisionValue::Marginal
    } else {
        SuitabilityDecisionValue::Meet
    };
    let selection_ready = required.iter().all(|dimension| {
        !matches!(
            dimension.decision,
            SuitabilityDecisionValue::Fail | SuitabilityDecisionValue::Unknown
        )
    });
    let mut blocked_claims = vec![
        claim::PRODUCTION_READY.to_string(),
        claim::TARGET_SELECTION_PI4_SUFFICIENT.to_string(),
        claim::BATTERY_SAFE.to_string(),
        claim::REAL_TIME_PRESSURE_SAFE.to_string(),
    ];
    if !selection_ready {
        blocked_claims.push(claim::SELECTION_READY.to_string());
    }
    blocked_claims.extend(target_contract.payload.blocked_claims.clone());
    blocked_claims.sort();
    blocked_claims.dedup();
    let mut data_quality = workload.data_quality.clone();
    if !target_evidence.missing.is_empty() {
        data_quality.degraded = true;
        data_quality.missing.extend(target_evidence.missing.clone());
        data_quality.notes.extend(
            target_evidence
                .missing
                .iter()
                .map(|missing| format!("missing target run numeric evidence: {missing}")),
        );
    }
    data_quality.notes.push(
        "suitability decision read target run numeric evidence and v2 operating contract claim IDs"
            .to_string(),
    );
    let mut evidence_refs = vec![
        context.target_contract_ref,
        context.workload_ref,
        context.policy_ref,
    ];
    evidence_refs.extend(target_contract.evidence_refs.clone());
    evidence_refs.extend(
        target_contract
            .payload
            .evaluations
            .iter()
            .flat_map(|evaluation| evaluation.evidence_refs.clone()),
    );
    evidence_refs.extend(target_evidence.evidence_refs);
    evidence_refs.sort();
    evidence_refs.dedup();
    let evaluations =
        suitability_policy_evaluations(&blocked_claims, selection_ready, &evidence_refs);
    let mut next_evidence = suitability_next_evidence(&evaluations);
    next_evidence.extend(target_contract.payload.next_evidence.clone());
    next_evidence.sort();
    next_evidence.dedup();
    let mut artifact = Artifact::new(
        Kind::ReportSuitability,
        new_id("SUITABILITY"),
        context.run_id,
        workload.target_id.clone(),
        if selection_ready {
            Status::MeasuredPartial
        } else {
            Status::Insufficient
        },
        SuitabilityPayload {
            rule_set_id: "rules.suitability.v2.policy_projection".to_string(),
            selection_ready,
            workload_id: Some(workload.workload_id.clone()),
            policy_id: Some(policy.policy_id.clone()),
            overall_decision: Some(overall_decision),
            dimensions,
            evaluations,
            blocked_claims,
            next_evidence,
        },
        now_unix_ms(),
    );
    artifact.claims = artifact
        .payload
        .evaluations
        .iter()
        .map(claim_for_evaluation)
        .collect();
    artifact.evidence_refs = evidence_refs;
    artifact.data_quality = DataQuality {
        level: if data_quality.degraded {
            DataQualityLevel::Degraded
        } else {
            DataQualityLevel::Partial
        },
        notes: data_quality.notes,
    };
    Ok(artifact)
}

pub fn generate_constraints_artifact_v2(
    suitability: &Artifact<SuitabilityPayload>,
) -> Artifact<ConstraintsPayload> {
    let payload = constraints_payload_from_suitability(suitability);
    let mut artifact = Artifact::new(
        Kind::ReportConstraints,
        new_id("CONSTRAINTS"),
        suitability.run_id.clone(),
        suitability.target_id.clone(),
        if payload.blocked_claims.is_empty() {
            Status::MeasuredPartial
        } else {
            Status::Insufficient
        },
        payload,
        now_unix_ms(),
    );
    artifact.evidence_refs = constraints_evidence_refs(suitability);
    artifact.claims = artifact
        .payload
        .blocked_claims
        .iter()
        .map(|claim_id| Claim {
            claim_id: claim_id.clone(),
            decision: Decision::Blocked,
            evidence_refs: artifact.evidence_refs.clone(),
            next_evidence: claim_definition(claim_id)
                .map(|definition| {
                    definition
                        .default_next_evidence
                        .iter()
                        .map(|item| (*item).to_string())
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();
    artifact.data_quality = DataQuality {
        level: DataQualityLevel::Partial,
        notes: vec!["v2 constraints are generated from v2 suitability claim IDs".to_string()],
    };
    artifact
}

fn constraints_evidence_refs(suitability: &Artifact<SuitabilityPayload>) -> Vec<String> {
    let mut refs = suitability.evidence_refs.clone();
    refs.extend(
        suitability
            .payload
            .evaluations
            .iter()
            .flat_map(|evaluation| evaluation.evidence_refs.clone()),
    );
    refs.sort();
    refs.dedup();
    refs
}

fn constraints_payload_from_suitability(
    suitability: &Artifact<SuitabilityPayload>,
) -> ConstraintsPayload {
    let mut allowed_patterns = Vec::new();
    let mut burst_only_patterns = vec!["bounded burst workload execution only".to_string()];
    let mut degraded_mode_triggers = Vec::new();
    let mut forbidden_patterns = vec![
        "unbounded all-core default loop".to_string(),
        "unbounded workload execution without duration and output limits".to_string(),
    ];
    let mut budget_constraints = Vec::new();
    let mut required_runtime_guards = vec![
        "preserve workload duration limits".to_string(),
        "preserve stdout/stderr byte limits".to_string(),
        "do not use SSH workload execution in v1".to_string(),
    ];
    for dimension in &suitability.payload.dimensions {
        match dimension.decision {
            SuitabilityDecisionValue::Meet => allowed_patterns.push(format!(
                "{:?} demand is within measured v2 policy envelope",
                dimension.dimension
            )),
            SuitabilityDecisionValue::Marginal => {
                burst_only_patterns.push(format!("{:?} remains burst-only", dimension.dimension));
                degraded_mode_triggers.push(format!(
                    "{:?} margin enters marginal band: {}",
                    dimension.dimension,
                    dimension
                        .margin
                        .clone()
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }
            SuitabilityDecisionValue::Fail => {
                forbidden_patterns.push(format!(
                    "default design relying on failed {:?} suitability",
                    dimension.dimension
                ));
                degraded_mode_triggers.push(format!(
                    "{:?} suitability fails under current evidence",
                    dimension.dimension
                ));
            }
            SuitabilityDecisionValue::Unknown => {
                required_runtime_guards.push(format!(
                    "collect evidence before relying on {:?} suitability",
                    dimension.dimension
                ));
            }
        }
        if let Some(margin) = &dimension.margin {
            budget_constraints.push(format!("{:?}: {margin}", dimension.dimension));
        }
    }
    allowed_patterns.sort();
    allowed_patterns.dedup();
    burst_only_patterns.sort();
    burst_only_patterns.dedup();
    degraded_mode_triggers.sort();
    degraded_mode_triggers.dedup();
    forbidden_patterns.sort();
    forbidden_patterns.dedup();
    budget_constraints.sort();
    budget_constraints.dedup();
    required_runtime_guards.sort();
    required_runtime_guards.dedup();
    let mut agent_instructions = vec![
        "Do not claim production readiness from this v2 workload suitability slice.".to_string(),
        "Treat unknown suitability dimensions as blocked until new evidence exists.".to_string(),
    ];
    if !suitability.payload.selection_ready {
        agent_instructions.push(
            "Do not select this target/workload pair as ready without resolving required unknowns or failures."
                .to_string(),
        );
    }
    ConstraintsPayload {
        source_suitability_id: suitability.id.clone(),
        workload_id: suitability.payload.workload_id.clone(),
        policy_id: suitability.payload.policy_id.clone(),
        allowed_patterns,
        burst_only_patterns,
        degraded_mode_triggers,
        forbidden_patterns,
        budget_constraints,
        required_runtime_guards,
        blocked_claims: suitability.payload.blocked_claims.clone(),
        agent_instructions,
        ci_rules: vec![
            "blocked_claims_must_not_appear_in_agent_facing_claims".to_string(),
            "unknown_required_dimensions_block_selection_readiness".to_string(),
        ],
    }
}

pub fn render_agent_constraints_markdown(
    constraints: &Artifact<ConstraintsPayload>,
    decision_ref: &str,
) -> String {
    let pack = &constraints.payload;
    let workload_id = pack.workload_id.as_deref().unwrap_or("unknown_workload");
    let mut out = String::new();
    out.push_str(&format!(
        "# Target Constraints for {} / {}\n\n",
        constraints.target_id, workload_id
    ));
    out.push_str("Source:\n");
    out.push_str(&format!("- suitability_artifact: {decision_ref}\n\n"));
    out.push_str("## Must obey\n\n");
    for instruction in &pack.agent_instructions {
        out.push_str(&format!("- {instruction}\n"));
    }
    for guard in &pack.required_runtime_guards {
        out.push_str(&format!("- {guard}\n"));
    }
    for trigger in &pack.degraded_mode_triggers {
        out.push_str(&format!("- Add degraded mode when {trigger}\n"));
    }
    out.push_str("\n## Budget constraints\n\n");
    if pack.budget_constraints.is_empty() {
        out.push_str("- No numeric budget constraint was established by this v2 decision.\n");
    } else {
        for constraint in &pack.budget_constraints {
            out.push_str(&format!("- {constraint}\n"));
        }
    }
    out.push_str("\n## Forbidden patterns\n\n");
    for pattern in &pack.forbidden_patterns {
        out.push_str(&format!("- {pattern}\n"));
    }
    out.push_str("\n## Blocked claims\n\n");
    for claim_id in &pack.blocked_claims {
        let blocked_claim = claim_definition(claim_id)
            .map(|definition| definition.blocked_claim)
            .unwrap_or(claim_id);
        out.push_str(&format!("- `{claim_id}`: \"{blocked_claim}\"\n"));
    }
    out
}

pub fn check_constraints_v2(
    constraints: &Artifact<ConstraintsPayload>,
    path: &Path,
    mode: ConstraintCheckMode,
) -> LabResult<Artifact<ConstraintCheckPayload>> {
    let mut matches = Vec::new();
    match mode {
        ConstraintCheckMode::CandidateContent => {
            scan_path_for_blocked_claims(&constraints.payload.blocked_claims, path, &mut matches)?;
        }
        ConstraintCheckMode::GeneratedConstraints => {
            check_generated_constraints(constraints, path, &mut matches)?;
        }
    }
    let payload = ConstraintCheckPayload {
        mode,
        status: if matches.is_empty() {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        checked_path: path.display().to_string(),
        matches,
    };
    let mut artifact = Artifact::new(
        Kind::ReportConstraintsCheck,
        new_id("CONSTRAINT-CHECK"),
        constraints.run_id.clone(),
        constraints.target_id.clone(),
        if payload.status == "pass" {
            Status::MeasuredPartial
        } else {
            Status::UnsafeBlocked {
                reason: "blocked constraints matched checked path".to_string(),
            }
        },
        payload,
        now_unix_ms(),
    );
    artifact.evidence_refs = constraints.evidence_refs.clone();
    Ok(artifact)
}

fn validate_policy(policy: &SuitabilityPolicy) -> LabResult<()> {
    if policy.schema_version != "lab.suitability_policy.v1" {
        return Err(LabError::Validation(
            "suitability policy schema_version must be lab.suitability_policy.v1".to_string(),
        ));
    }
    if !policy.rules.unknown_required_dimension_blocks_selection
        || !policy.rules.unknown_never_becomes_meet
    {
        return Err(LabError::Policy(
            "suitability policy cannot convert unknown evidence into meet".to_string(),
        ));
    }
    Ok(())
}

fn suitability_policy_evaluations(
    blocked_claims: &[String],
    selection_ready: bool,
    evidence_refs: &[String],
) -> Vec<RuleEvaluation> {
    let mut evaluations = blocked_claims
        .iter()
        .map(|claim_id| RuleEvaluation {
            rule_id: format!("suitability.policy.blocked.{}", rule_id_suffix(claim_id)),
            claim_id: claim_id.clone(),
            matched: false,
            decision: Decision::Blocked,
            evidence_refs: evidence_refs.to_vec(),
            missing: Vec::new(),
            next_evidence: next_evidence_for_claim_id(claim_id),
        })
        .collect::<Vec<_>>();
    if selection_ready {
        evaluations.push(RuleEvaluation {
            rule_id: "suitability.policy.selection_ready".to_string(),
            claim_id: claim::SELECTION_READY.to_string(),
            matched: true,
            decision: Decision::Provisional,
            evidence_refs: evidence_refs.to_vec(),
            missing: Vec::new(),
            next_evidence: Vec::new(),
        });
    }
    evaluations
}

fn next_evidence_for_claim_id(claim_id: &str) -> Vec<String> {
    claim_definition(claim_id)
        .map(|definition| {
            definition
                .default_next_evidence
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![format!("collect evidence to unblock: {claim_id}")])
}

fn suitability_next_evidence(evaluations: &[RuleEvaluation]) -> Vec<String> {
    let mut items = evaluations
        .iter()
        .flat_map(|evaluation| evaluation.next_evidence.clone())
        .collect::<Vec<_>>();
    items.sort();
    items.dedup();
    items
}

fn rule_id_suffix(claim_id: &str) -> String {
    claim_id
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn decide_dimension(
    dimension: SuitabilityDimensionKind,
    policy: &SuitabilityPolicy,
    workload: &WorkloadDemandProfile,
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    match dimension {
        SuitabilityDimensionKind::Cpu => decide_cpu(policy, workload, evidence_refs),
        SuitabilityDimensionKind::Thermal => {
            decide_thermal(policy, workload, target_evidence, evidence_refs)
        }
        SuitabilityDimensionKind::Memory => {
            decide_memory(policy, workload, target_evidence, evidence_refs)
        }
        SuitabilityDimensionKind::StorageIo => decide_storage_io(target_evidence, evidence_refs),
        SuitabilityDimensionKind::NetworkIo => decide_network_io(target_evidence, evidence_refs),
        SuitabilityDimensionKind::LatencyJitter => {
            decide_latency_jitter(target_evidence, evidence_refs)
        }
        other => unknown_dimension(
            other,
            "dimension is not measured by the local workload suitability v1 slice".to_string(),
            evidence_refs,
            false,
            true,
        ),
    }
}

fn decide_storage_io(
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let pressure = matching_pressure(target_evidence, "storage_io");
    if pressure.is_empty() {
        return unknown_dimension(
            SuitabilityDimensionKind::StorageIo,
            "storage_io pressure evidence is missing".to_string(),
            evidence_refs,
            true,
            false,
        );
    }
    let refs = refs_with_pressure(evidence_refs, &pressure);
    if pressure
        .iter()
        .any(|evidence| evidence.is_measuredish() && evidence.effect_observed)
    {
        return SuitabilityDimensionDecision {
            dimension: SuitabilityDimensionKind::StorageIo,
            decision: SuitabilityDecisionValue::Marginal,
            requirement:
                "storage_io evidence must be run-backed and device-specific before it can meet"
                    .to_string(),
            observed_demand: Some("bounded tempfile storage I/O smoke observed".to_string()),
            target_envelope: Some(
                "tempfile/page-cache evidence exists, but sustained device behavior is not established"
                    .to_string(),
            ),
            margin: Some("tempfile/page-cache smoke is not a storage-device meet".to_string()),
            confidence: SuitabilityConfidence::Low,
            target_conditioned: true,
            portable_between_targets: false,
            evidence_refs: refs,
            unknown_reason: None,
            next_evidence_needed: vec![
                "collect device-backed sustained storage I/O evidence for storage-device meet"
                    .to_string(),
            ],
        };
    }
    unknown_dimension(
        SuitabilityDimensionKind::StorageIo,
        "storage_io evidence is present but insufficient for suitability".to_string(),
        refs,
        true,
        false,
    )
}

fn decide_network_io(
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let pressure = matching_pressure(target_evidence, "network_io");
    if pressure.is_empty() {
        return unknown_dimension(
            SuitabilityDimensionKind::NetworkIo,
            "endpoint-backed network_io pressure evidence is missing".to_string(),
            evidence_refs,
            true,
            false,
        );
    }
    let refs = refs_with_pressure(evidence_refs, &pressure);
    if let Some(evidence) = pressure
        .iter()
        .find(|evidence| evidence.is_endpoint_backed_bounded_transfer())
    {
        return SuitabilityDimensionDecision {
            dimension: SuitabilityDimensionKind::NetworkIo,
            decision: SuitabilityDecisionValue::Marginal,
            requirement:
                "network_io evidence must be endpoint-backed bounded_transfer before it can support suitability"
                    .to_string(),
            observed_demand: Some(format!(
                "bounded_transfer generated {} bytes",
                evidence.network_traffic_generated_bytes.unwrap_or(0)
            )),
            target_envelope: Some(
                "endpoint-backed bounded transfer was observed under this run's network conditions"
                    .to_string(),
            ),
            margin: Some(
                "bounded transfer evidence present; no throughput, loss, or retry policy threshold supplied"
                    .to_string(),
            ),
            confidence: SuitabilityConfidence::Medium,
            target_conditioned: true,
            portable_between_targets: false,
            evidence_refs: refs,
            unknown_reason: None,
            next_evidence_needed: vec![
                "add policy thresholds for throughput, loss, retry, and endpoint topology before claiming network meet"
                    .to_string(),
            ],
        };
    }
    unknown_dimension(
        SuitabilityDimensionKind::NetworkIo,
        "network_io evidence is not endpoint-backed bounded_transfer; counter-only or failed endpoint evidence cannot support network suitability"
            .to_string(),
        refs,
        true,
        false,
    )
}

fn decide_latency_jitter(
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let pressure = matching_pressure(target_evidence, "latency_jitter");
    if pressure.is_empty() {
        return unknown_dimension(
            SuitabilityDimensionKind::LatencyJitter,
            "latency_jitter pressure evidence is missing".to_string(),
            evidence_refs,
            true,
            false,
        );
    }
    let refs = refs_with_pressure(evidence_refs, &pressure);
    if pressure.iter().any(|evidence| evidence.is_measuredish()) {
        return SuitabilityDimensionDecision {
            dimension: SuitabilityDimensionKind::LatencyJitter,
            decision: SuitabilityDecisionValue::Marginal,
            requirement:
                "latency_jitter evidence must be target-run backed and policy-thresholded before it can meet"
                    .to_string(),
            observed_demand: Some("current-condition monotonic jitter distribution measured".to_string()),
            target_envelope: Some(
                "current-condition jitter evidence exists, but no workload latency SLO is established"
                    .to_string(),
            ),
            margin: Some("latency evidence is present without a policy threshold".to_string()),
            confidence: SuitabilityConfidence::Low,
            target_conditioned: true,
            portable_between_targets: false,
            evidence_refs: refs,
            unknown_reason: None,
            next_evidence_needed: vec![
                "collect pressure-specific p95/p99 latency evidence with workload latency thresholds"
                    .to_string(),
            ],
        };
    }
    unknown_dimension(
        SuitabilityDimensionKind::LatencyJitter,
        "latency_jitter evidence is present but insufficient for suitability".to_string(),
        refs,
        true,
        false,
    )
}

fn decide_cpu(
    policy: &SuitabilityPolicy,
    workload: &WorkloadDemandProfile,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let Some(cpu_policy) = policy.cpu.as_ref() else {
        return unknown_dimension(
            SuitabilityDimensionKind::Cpu,
            "cpu policy threshold is missing".to_string(),
            evidence_refs,
            false,
            true,
        );
    };
    if workload.data_quality.degraded {
        return unknown_dimension(
            SuitabilityDimensionKind::Cpu,
            "workload run did not complete cleanly; steady demand cannot be established"
                .to_string(),
            evidence_refs,
            false,
            true,
        );
    }
    let Some(avg) = workload.workload_demand.process_cpu_percent_avg else {
        return unknown_dimension(
            SuitabilityDimensionKind::Cpu,
            "process-scoped CPU demand is unavailable".to_string(),
            evidence_refs,
            false,
            true,
        );
    };
    let margin = cpu_policy.max_process_cpu_percent_avg - avg;
    let decision = if margin < 0.0 {
        SuitabilityDecisionValue::Fail
    } else if margin <= cpu_policy.max_process_cpu_percent_avg * 0.10 {
        SuitabilityDecisionValue::Marginal
    } else {
        SuitabilityDecisionValue::Meet
    };
    SuitabilityDimensionDecision {
        dimension: SuitabilityDimensionKind::Cpu,
        decision,
        requirement: format!(
            "process_cpu_percent_avg <= {:.1}",
            cpu_policy.max_process_cpu_percent_avg
        ),
        observed_demand: Some(format!("{avg:.2}% process CPU avg")),
        target_envelope: Some("policy CPU envelope from suitability policy".to_string()),
        margin: Some(format!("{margin:.2} percentage points")),
        confidence: SuitabilityConfidence::Medium,
        target_conditioned: false,
        portable_between_targets: true,
        evidence_refs,
        unknown_reason: None,
        next_evidence_needed: Vec::new(),
    }
}

fn decide_thermal(
    policy: &SuitabilityPolicy,
    workload: &WorkloadDemandProfile,
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let Some(thermal_policy) = policy.thermal.as_ref() else {
        return unknown_dimension(
            SuitabilityDimensionKind::Thermal,
            "thermal policy threshold is missing".to_string(),
            evidence_refs,
            true,
            false,
        );
    };
    if workload
        .target_conditioned_response
        .abort_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("thermal"))
    {
        return SuitabilityDimensionDecision {
            dimension: SuitabilityDimensionKind::Thermal,
            decision: SuitabilityDecisionValue::Fail,
            requirement: format!("thermal_max_c <= {:.1}", thermal_policy.max_temp_c),
            observed_demand: workload
                .target_conditioned_response
                .thermal_max_c
                .map(|temp| format!("{temp:.1}C")),
            target_envelope: Some("target-conditioned thermal response".to_string()),
            margin: workload
                .target_conditioned_response
                .thermal_max_c
                .map(|temp| format!("{:.1}C", thermal_policy.max_temp_c - temp)),
            confidence: SuitabilityConfidence::Medium,
            target_conditioned: true,
            portable_between_targets: false,
            evidence_refs,
            unknown_reason: None,
            next_evidence_needed: vec![
                "completed bounded workload run below thermal abort".to_string()
            ],
        };
    }
    let observed = workload
        .target_conditioned_response
        .thermal_max_c
        .or(target_evidence.max_temp_c);
    let Some(temp) = observed else {
        return unknown_dimension(
            SuitabilityDimensionKind::Thermal,
            "thermal response was not visible in workload or target run evidence".to_string(),
            evidence_refs,
            true,
            false,
        );
    };
    let margin = thermal_policy.max_temp_c - temp;
    let decision = if margin < 0.0 {
        SuitabilityDecisionValue::Fail
    } else if margin < thermal_policy.marginal_margin_c_below {
        SuitabilityDecisionValue::Marginal
    } else {
        SuitabilityDecisionValue::Meet
    };
    SuitabilityDimensionDecision {
        dimension: SuitabilityDimensionKind::Thermal,
        decision,
        requirement: format!("thermal_max_c <= {:.1}", thermal_policy.max_temp_c),
        observed_demand: Some(format!("{temp:.1}C target-conditioned response")),
        target_envelope: Some("run-backed target thermal evidence".to_string()),
        margin: Some(format!("{margin:.1}C")),
        confidence: SuitabilityConfidence::Medium,
        target_conditioned: true,
        portable_between_targets: false,
        evidence_refs,
        unknown_reason: None,
        next_evidence_needed: Vec::new(),
    }
}

fn decide_memory(
    policy: &SuitabilityPolicy,
    workload: &WorkloadDemandProfile,
    target_evidence: &TargetRunNumericEvidence,
    evidence_refs: Vec<String>,
) -> SuitabilityDimensionDecision {
    let Some(memory_policy) = policy.memory.as_ref() else {
        return unknown_dimension(
            SuitabilityDimensionKind::Memory,
            "memory policy threshold is missing".to_string(),
            evidence_refs,
            false,
            true,
        );
    };
    if workload.data_quality.degraded {
        return unknown_dimension(
            SuitabilityDimensionKind::Memory,
            "workload run did not complete cleanly; memory margin cannot be trusted".to_string(),
            evidence_refs,
            false,
            true,
        );
    }
    let Some(rss_peak_kb) = workload.workload_demand.rss_peak_kb else {
        return unknown_dimension(
            SuitabilityDimensionKind::Memory,
            "process RSS demand is unavailable".to_string(),
            evidence_refs,
            false,
            true,
        );
    };
    let available_kb = workload
        .system_context
        .system_memory_available_min_kb
        .or(target_evidence.memory_available_min_kb);
    let Some(available_kb) = available_kb else {
        return unknown_dimension(
            SuitabilityDimensionKind::Memory,
            "memory availability evidence is missing".to_string(),
            evidence_refs,
            false,
            true,
        );
    };
    let available_mb = available_kb as f64 / 1024.0;
    let margin = available_mb - memory_policy.min_memory_margin_mb as f64;
    let decision = if margin < 0.0 {
        SuitabilityDecisionValue::Fail
    } else if margin < 128.0 {
        SuitabilityDecisionValue::Marginal
    } else {
        SuitabilityDecisionValue::Meet
    };
    SuitabilityDimensionDecision {
        dimension: SuitabilityDimensionKind::Memory,
        decision,
        requirement: format!(
            "system_memory_available_min_mb >= {}",
            memory_policy.min_memory_margin_mb
        ),
        observed_demand: Some(format!("{:.1}MiB RSS peak", rss_peak_kb as f64 / 1024.0)),
        target_envelope: Some(format!(
            "{available_mb:.1}MiB minimum available memory observed"
        )),
        margin: Some(format!("{margin:.1}MiB")),
        confidence: SuitabilityConfidence::Medium,
        target_conditioned: false,
        portable_between_targets: true,
        evidence_refs,
        unknown_reason: None,
        next_evidence_needed: Vec::new(),
    }
}

fn matching_pressure<'a>(
    target_evidence: &'a TargetRunNumericEvidence,
    kind: &str,
) -> Vec<&'a TargetRunPressureEvidence> {
    target_evidence
        .pressure
        .iter()
        .filter(|evidence| evidence.kind == kind)
        .collect()
}

fn refs_with_pressure(
    mut evidence_refs: Vec<String>,
    pressure: &[&TargetRunPressureEvidence],
) -> Vec<String> {
    evidence_refs.extend(
        pressure
            .iter()
            .map(|evidence| evidence.artifact_ref.clone()),
    );
    evidence_refs.sort();
    evidence_refs.dedup();
    evidence_refs
}

fn unknown_dimension(
    dimension: SuitabilityDimensionKind,
    reason: String,
    evidence_refs: Vec<String>,
    target_conditioned: bool,
    portable_between_targets: bool,
) -> SuitabilityDimensionDecision {
    SuitabilityDimensionDecision {
        dimension,
        decision: SuitabilityDecisionValue::Unknown,
        requirement: "required evidence must be present".to_string(),
        observed_demand: None,
        target_envelope: None,
        margin: None,
        confidence: SuitabilityConfidence::Low,
        target_conditioned,
        portable_between_targets,
        evidence_refs,
        unknown_reason: Some(reason.clone()),
        next_evidence_needed: vec![reason],
    }
}

fn collect_target_run_numeric_evidence(run_dir: &Path) -> TargetRunNumericEvidence {
    let mut evidence = TargetRunNumericEvidence::default();
    if !run_dir.exists() {
        evidence.missing.push(format!(
            "target run directory not found: {}",
            run_dir.display()
        ));
        return evidence;
    }
    collect_json_values(run_dir, run_dir, &mut evidence);
    if evidence.max_temp_c.is_none() {
        evidence
            .missing
            .push("target run thermal numeric evidence not found".to_string());
    }
    if evidence.memory_available_min_kb.is_none() {
        evidence
            .missing
            .push("target run memory availability numeric evidence not found".to_string());
    }
    evidence
}

fn collect_json_values(root: &Path, path: &Path, evidence: &mut TargetRunNumericEvidence) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_values(root, &path, evidence);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            if let Ok(value) = read_json::<Value>(&path) {
                if let Ok(relative) = path.strip_prefix(root) {
                    let artifact_ref = format!("target-run://{}", relative.display());
                    harvest_pressure_value(&value, &artifact_ref, evidence);
                    evidence.evidence_refs.push(artifact_ref);
                }
                harvest_numeric_value(&value, evidence);
            }
        }
    }
}

fn harvest_pressure_value(
    value: &Value,
    artifact_ref: &str,
    evidence: &mut TargetRunNumericEvidence,
) {
    let Some(map) = value.as_object() else {
        return;
    };
    if map.get("kind").and_then(|value| value.as_str()) != Some("pressure") {
        return;
    }
    let Some(payload) = map.get("payload").and_then(|value| value.as_object()) else {
        return;
    };
    let Some(kind) = payload
        .get("pressure_kind")
        .and_then(|value| value.as_str())
    else {
        return;
    };
    evidence.pressure.push(TargetRunPressureEvidence {
        kind: kind.to_string(),
        status_state: map
            .get("status")
            .and_then(|value| value.get("state"))
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        evidence_class: payload
            .get("evidence_class")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        effect_observed: payload
            .get("effect_observed")
            .and_then(|value| value.as_bool())
            .unwrap_or(false),
        network_mode: payload
            .get("network_mode")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
        network_endpoint_available: payload
            .get("network_endpoint_available")
            .and_then(|value| value.as_bool()),
        network_traffic_generated_bytes: payload
            .get("network_traffic_generated_bytes")
            .and_then(|value| value.as_u64()),
        artifact_ref: artifact_ref.to_string(),
    });
}

fn harvest_numeric_value(value: &Value, evidence: &mut TargetRunNumericEvidence) {
    match value {
        Value::Object(map) => {
            if let Some(temp) = map
                .get("max_observed_temp_c")
                .or_else(|| map.get("thermal_max_c"))
                .and_then(|value| value.as_f64())
            {
                evidence.max_temp_c = Some(
                    evidence
                        .max_temp_c
                        .map_or(temp, |current| current.max(temp)),
                );
            }
            if let Some(mem) = map
                .get("memory_available_kb")
                .or_else(|| map.get("memory_available_kb_min"))
                .or_else(|| map.get("system_memory_available_min_kb"))
                .and_then(|value| value.as_u64())
            {
                evidence.memory_available_min_kb = Some(
                    evidence
                        .memory_available_min_kb
                        .map_or(mem, |current| current.min(mem)),
                );
            }
            if map
                .get("metric_id")
                .and_then(|value| value.as_str())
                .is_some_and(|id| id.contains("temp"))
            {
                if let Some(temp) = map.get("value").and_then(|value| value.as_f64()) {
                    evidence.max_temp_c = Some(
                        evidence
                            .max_temp_c
                            .map_or(temp, |current| current.max(temp)),
                    );
                }
            }
            for child in map.values() {
                harvest_numeric_value(child, evidence);
            }
        }
        Value::Array(values) => {
            for child in values {
                harvest_numeric_value(child, evidence);
            }
        }
        _ => {}
    }
}

fn scan_path_for_blocked_claims(
    blocked_claims: &[String],
    path: &Path,
    matches: &mut Vec<ConstraintCheckMatch>,
) -> LabResult<()> {
    if path.is_dir() {
        for entry in fs::read_dir(path).map_err(|source| LabError::IoWithPath {
            path: path.to_path_buf(),
            source,
        })? {
            let entry = entry?;
            let child = entry.path();
            if should_skip_path(&child) {
                continue;
            }
            scan_path_for_blocked_claims(blocked_claims, &child, matches)?;
        }
    } else if should_scan_file(path) {
        let Ok(text) = fs::read_to_string(path) else {
            return Ok(());
        };
        for claim_id in blocked_claims {
            let blocked_claim = claim_definition(claim_id)
                .map(|definition| definition.blocked_claim)
                .unwrap_or(claim_id);
            if !blocked_claim.trim().is_empty() && text.contains(blocked_claim) {
                matches.push(ConstraintCheckMatch {
                    path: path.display().to_string(),
                    claim_id: claim_id.clone(),
                    blocked_claim: blocked_claim.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn check_generated_constraints(
    constraints: &Artifact<ConstraintsPayload>,
    path: &Path,
    matches: &mut Vec<ConstraintCheckMatch>,
) -> LabResult<()> {
    if path.is_dir() {
        push_generated_constraint_mismatch(
            path,
            "generated constraints self-check expects a generated file path",
            matches,
        );
        return Ok(());
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(_) => {
            push_generated_constraint_mismatch(
                path,
                "generated constraints self-check could not read the file",
                matches,
            );
            return Ok(());
        }
    };
    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
        check_generated_constraints_json(constraints, path, &text, matches)?;
    } else {
        check_generated_constraints_markdown(constraints, path, &text, matches);
    }
    Ok(())
}

fn check_generated_constraints_json(
    expected: &Artifact<ConstraintsPayload>,
    path: &Path,
    text: &str,
    matches: &mut Vec<ConstraintCheckMatch>,
) -> LabResult<()> {
    let generated: Artifact<ConstraintsPayload> = match serde_json::from_str(text) {
        Ok(generated) => generated,
        Err(_) => {
            push_generated_constraint_mismatch(
                path,
                "generated constraints JSON is not a report.constraints artifact",
                matches,
            );
            return Ok(());
        }
    };
    for (label, expected_value, actual_value) in [
        (
            "schema",
            expected.schema.as_str(),
            generated.schema.as_str(),
        ),
        (
            "kind",
            "report.constraints",
            match generated.kind {
                Kind::ReportConstraints => "report.constraints",
                _ => "other",
            },
        ),
        ("id", expected.id.as_str(), generated.id.as_str()),
        (
            "run_id",
            expected.run_id.as_str(),
            generated.run_id.as_str(),
        ),
        (
            "target_id",
            expected.target_id.as_str(),
            generated.target_id.as_str(),
        ),
        (
            "source_suitability_id",
            expected.payload.source_suitability_id.as_str(),
            generated.payload.source_suitability_id.as_str(),
        ),
    ] {
        if expected_value != actual_value {
            push_generated_constraint_mismatch(
                path,
                &format!(
                    "generated constraints JSON {label} mismatch: expected {expected_value}, got {actual_value}"
                ),
                matches,
            );
        }
    }
    if expected.payload.blocked_claims != generated.payload.blocked_claims {
        push_generated_constraint_mismatch(
            path,
            "generated constraints JSON blocked_claims mismatch",
            matches,
        );
    }
    Ok(())
}

fn check_generated_constraints_markdown(
    constraints: &Artifact<ConstraintsPayload>,
    path: &Path,
    text: &str,
    matches: &mut Vec<ConstraintCheckMatch>,
) {
    for required in [
        "# Target Constraints",
        "Source:",
        "## Must obey",
        "## Blocked claims",
    ] {
        if !text.contains(required) {
            push_generated_constraint_mismatch(
                path,
                &format!("generated constraints markdown is missing section {required}"),
                matches,
            );
        }
    }
    for claim_id in &constraints.payload.blocked_claims {
        if !text.contains(claim_id) {
            push_generated_constraint_mismatch(
                path,
                &format!("generated constraints markdown is missing blocked claim id {claim_id}"),
                matches,
            );
        }
    }
}

fn push_generated_constraint_mismatch(
    path: &Path,
    reason: &str,
    matches: &mut Vec<ConstraintCheckMatch>,
) {
    matches.push(ConstraintCheckMatch {
        path: path.display().to_string(),
        claim_id: "report.constraints".to_string(),
        blocked_claim: reason.to_string(),
    });
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, ".git" | "target" | "lab"))
}

fn should_scan_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext, "md" | "txt" | "rs" | "json" | "yaml" | "yml"))
}
