use crate::evidence::{
    claim, Artifact, DataQuality, DataQualityLevel, Decision, EvidenceStore, Kind, Status,
};
use crate::ids::{new_id, now_unix_ms};
use crate::rules::engine::{claim_for_evaluation, evaluate_rules, Pred, Rule, RuleEvaluation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuitabilityPayload {
    pub rule_set_id: String,
    pub selection_ready: bool,
    pub evaluations: Vec<RuleEvaluation>,
    pub blocked_claims: Vec<String>,
    pub next_evidence: Vec<String>,
}

pub fn suitability_rules() -> Vec<Rule> {
    vec![
        Rule {
            id: "suitability.requires_operating_contract_and_workload",
            claim_id: claim::TARGET_SELECTION_PI4_SUFFICIENT,
            when: Pred::All(vec![
                Pred::Present(Kind::ReportOperatingContract),
                Pred::Present(Kind::Workload),
            ]),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::ReportOperatingContract, Kind::Workload],
            next_evidence: &["evaluate workload demand against v2 operating contract"],
        },
        Rule {
            id: "suitability.production_readiness_requires_suitability_evidence",
            claim_id: claim::PRODUCTION_READY,
            when: Pred::All(vec![
                Pred::Present(Kind::ReportOperatingContract),
                Pred::Present(Kind::Workload),
                Pred::Present(Kind::ReportSuitability),
            ]),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[
                Kind::ReportOperatingContract,
                Kind::Workload,
                Kind::ReportSuitability,
            ],
            next_evidence: &["record v2 suitability artifact before any readiness claim"],
        },
    ]
}

pub fn evaluate_suitability_v2(
    store: &EvidenceStore,
    run_id: impl Into<String>,
    target_id: impl Into<String>,
) -> Artifact<SuitabilityPayload> {
    suitability_from_rules_v2(
        store,
        "rules.suitability.v2",
        suitability_rules(),
        run_id,
        target_id,
    )
}

pub fn suitability_from_rules_v2(
    store: &EvidenceStore,
    rule_set_id: impl Into<String>,
    rules: Vec<Rule>,
    run_id: impl Into<String>,
    target_id: impl Into<String>,
) -> Artifact<SuitabilityPayload> {
    let rule_set_id = rule_set_id.into();
    let evaluations = evaluate_rules(store, &rules);
    let selection_ready = evaluations
        .iter()
        .filter(|evaluation| evaluation.claim_id == claim::TARGET_SELECTION_PI4_SUFFICIENT)
        .all(|evaluation| evaluation.decision != Decision::Blocked);
    let status = if selection_ready {
        Status::MeasuredPartial
    } else {
        Status::Insufficient
    };
    let blocked_claims = blocked_claims(&evaluations);
    let next_evidence = next_evidence(&evaluations);
    let mut artifact = Artifact::new(
        Kind::ReportSuitability,
        new_id("SUITABILITY"),
        run_id,
        target_id,
        status,
        SuitabilityPayload {
            rule_set_id,
            selection_ready,
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
    artifact.data_quality = DataQuality {
        level: if selection_ready {
            DataQualityLevel::Complete
        } else {
            DataQualityLevel::Partial
        },
        notes: vec!["v2 suitability evaluated from rule table".to_string()],
    };
    artifact
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
