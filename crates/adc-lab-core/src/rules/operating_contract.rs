use crate::evidence::{
    claim, claim_definition, Artifact, DataQuality, DataQualityLevel, Decision, EvidenceStore,
    Kind, Status,
};
use crate::ids::new_id;
use crate::rules::engine::{claim_for_evaluation, evaluate_rules, Pred, Rule, RuleEvaluation};
use crate::{
    ContractConfidence, OperatingContractRule, OperatingRuleCategory, OperatingRuleSource,
    TargetOperatingContract, TargetOperatingContractStatus,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OperatingContractPayload {
    pub rule_set_id: String,
    pub evaluations: Vec<RuleEvaluation>,
    pub blocked_claims: Vec<String>,
    pub next_evidence: Vec<String>,
}

pub fn operating_contract_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "operating.memory_storage_coupling_requires_composite",
            claim_id: claim::COUPLING_MEMORY_TO_STORAGE,
            when: Pred::All(vec![
                Pred::PressureEffect("memory_pressure"),
                Pred::CompositeMeasured("memory_storage_jitter"),
            ]),
            on_match: Decision::Supported,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure, Kind::Composite],
            next_evidence: &["run paired memory/storage composite probe"],
        },
        Rule {
            id: "operating.production_readiness_requires_run_report",
            claim_id: claim::PRODUCTION_READY,
            when: Pred::Present(Kind::ReportRun),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::ReportRun],
            next_evidence: &["generate v2 run report from the same operation sequence"],
        },
    ]
}

pub fn evaluate_operating_contract_v2(
    store: &EvidenceStore,
    run_id: impl Into<String>,
    target_id: impl Into<String>,
) -> Artifact<OperatingContractPayload> {
    operating_contract_from_rules_v2(
        store,
        "rules.operating_contract.v2",
        operating_contract_rules(),
        run_id,
        target_id,
    )
}

pub fn operating_contract_from_rules_v2(
    store: &EvidenceStore,
    rule_set_id: impl Into<String>,
    rules: Vec<Rule>,
    run_id: impl Into<String>,
    target_id: impl Into<String>,
) -> Artifact<OperatingContractPayload> {
    let rule_set_id = rule_set_id.into();
    let evaluations = evaluate_rules(store, &rules);
    let status = status_for_evaluations(&evaluations);
    let blocked_claims = blocked_claims(&evaluations);
    let next_evidence = next_evidence(&evaluations);
    let mut artifact = Artifact::new(
        Kind::ReportOperatingContract,
        new_id("OPERATING-CONTRACT"),
        run_id,
        target_id,
        status,
        OperatingContractPayload {
            rule_set_id,
            evaluations,
            blocked_claims,
            next_evidence,
        },
        crate::ids::now_unix_ms(),
    );
    artifact.claims = artifact
        .payload
        .evaluations
        .iter()
        .map(claim_for_evaluation)
        .collect();
    artifact.data_quality = DataQuality {
        level: if artifact
            .payload
            .evaluations
            .iter()
            .all(|evaluation| evaluation.matched)
        {
            DataQualityLevel::Complete
        } else {
            DataQualityLevel::Partial
        },
        notes: vec!["v2 operating contract evaluated from rule table".to_string()],
    };
    artifact
}

pub fn legacy_contract_from_v2_artifact(
    artifact: &Artifact<OperatingContractPayload>,
    target_class: impl Into<String>,
) -> TargetOperatingContract {
    TargetOperatingContract {
        schema_version: "lab.target_operating_contract.v1.projected_from_v2".to_string(),
        target_id: artifact.target_id.clone(),
        target_class: target_class.into(),
        contract_status: legacy_contract_status(&artifact.status),
        rules: artifact
            .payload
            .evaluations
            .iter()
            .filter(|evaluation| evaluation.decision == Decision::Blocked)
            .map(legacy_blocked_rule)
            .collect(),
        boundaries: Vec::new(),
        unknowns: Vec::new(),
        next_evidence_needed: artifact.payload.next_evidence.clone(),
        time_unix_ms: artifact.time_unix_ms,
    }
}

fn status_for_evaluations(evaluations: &[RuleEvaluation]) -> Status {
    if evaluations
        .iter()
        .all(|evaluation| evaluation.decision != Decision::Blocked)
    {
        Status::MeasuredPartial
    } else {
        Status::Insufficient
    }
}

fn legacy_contract_status(status: &Status) -> TargetOperatingContractStatus {
    match status {
        Status::Measured | Status::MeasuredPartial => {
            TargetOperatingContractStatus::MeasuredPartial
        }
        Status::Insufficient
        | Status::NotApplicable { .. }
        | Status::Refused { .. }
        | Status::UnsafeBlocked { .. } => TargetOperatingContractStatus::Insufficient,
    }
}

fn legacy_blocked_rule(evaluation: &RuleEvaluation) -> OperatingContractRule {
    OperatingContractRule {
        rule_id: evaluation.rule_id.clone(),
        category: OperatingRuleCategory::BlockedClaim,
        statement: claim_definition(&evaluation.claim_id)
            .map(|definition| definition.blocked_claim.to_string())
            .unwrap_or_else(|| evaluation.claim_id.clone()),
        rule_source: OperatingRuleSource::EvidenceNeededRule,
        derivation: "projected from v2 operating contract evaluation".to_string(),
        evidence_refs: evaluation.evidence_refs.clone(),
        confidence: ContractConfidence::Low,
        allowed_design: Vec::new(),
        blocked_design: evaluation.next_evidence.clone(),
    }
}

fn blocked_claims(evaluations: &[RuleEvaluation]) -> Vec<String> {
    let mut claims = evaluations
        .iter()
        .filter(|evaluation| evaluation.decision == Decision::Blocked)
        .map(|evaluation| evaluation.claim_id.clone())
        .collect::<Vec<_>>();
    claims.sort();
    claims.dedup();
    claims
}

fn next_evidence(evaluations: &[RuleEvaluation]) -> Vec<String> {
    let mut items = evaluations
        .iter()
        .flat_map(|evaluation| evaluation.next_evidence.clone())
        .collect::<Vec<_>>();
    items.sort();
    items.dedup();
    items
}
