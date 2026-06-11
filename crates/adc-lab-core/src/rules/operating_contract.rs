use crate::evidence::{
    claim, Artifact, DataQuality, DataQualityLevel, Decision, EvidenceStore, Kind, Status,
};
use crate::ids::new_id;
use crate::rules::engine::{claim_for_evaluation, evaluate_rules, Pred, Rule, RuleEvaluation};
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
            id: "operating.sustained_thermal_requires_soak",
            claim_id: claim::THERMAL_SUSTAINED_SOAK,
            when: Pred::All(vec![
                Pred::LoadDurationAtLeastSeconds(900),
                Pred::PressureEffect("thermal_pressure"),
            ]),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Load, Kind::Pressure],
            next_evidence: &["run approved sustained thermal soak with cooldown observation"],
        },
        Rule {
            id: "operating.storage_default_writes_require_bounded_probe",
            claim_id: claim::STORAGE_DEFAULT_WRITES_BOUNDED,
            when: Pred::PressureEffect("storage_io"),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure],
            next_evidence: &["run bounded storage I/O probe with cleanup verification"],
        },
        Rule {
            id: "operating.network_background_io_requires_bounded_transfer",
            claim_id: claim::NETWORK_BOUNDED_TRANSFER,
            when: Pred::All(vec![
                Pred::PressureEffect("network_io"),
                Pred::NetworkBoundedTransfer,
            ]),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure],
            next_evidence: &["run endpoint-backed bounded network transfer"],
        },
        Rule {
            id: "operating.real_time_pressure_requires_jitter_evidence",
            claim_id: claim::REAL_TIME_PRESSURE_SAFE,
            when: Pred::PressureEffect("latency_jitter"),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure],
            next_evidence: &["run pressure-specific jitter probe and record p95/p99/max"],
        },
        Rule {
            id: "operating.observer_cadence_requires_bounded_samples",
            claim_id: claim::OBSERVER_CADENCE_BOUNDED,
            when: Pred::Any(vec![
                Pred::PressureEffect("observer_pressure"),
                Pred::ObservationSamplesAtLeast(2),
            ]),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Observation, Kind::Pressure],
            next_evidence: &["record bounded observer cadence or observer pressure evidence"],
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
