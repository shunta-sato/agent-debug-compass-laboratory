use crate::evidence::{
    claim_definition, ArtifactMeta, Claim, Decision, EvidenceStore, Kind, Status,
};
use crate::probe::{CompositePayload, LoadPayload, ObservationPayload, PressurePayload};
use crate::run_validation::{
    is_measured_fullset_validation, RunValidationPayload, FULLSET_PROFILE,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum Pred {
    Present(Kind),
    PressureEffect(&'static str),
    CompositeMeasured(&'static str),
    LoadDurationAtLeastSeconds(u64),
    NetworkBoundedTransfer,
    ObservationSamplesAtLeast(usize),
    RunValidationMeasured,
    All(Vec<Pred>),
    Any(Vec<Pred>),
    Not(Box<Pred>),
    Custom(&'static str, fn(&EvidenceStore) -> bool),
}

#[derive(Debug, Clone)]
pub struct Rule {
    pub id: &'static str,
    pub claim_id: &'static str,
    pub when: Pred,
    pub on_match: Decision,
    pub on_miss: Decision,
    pub evidence_kinds: &'static [Kind],
    pub next_evidence: &'static [&'static str],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuleEvaluation {
    pub rule_id: String,
    pub claim_id: String,
    pub matched: bool,
    pub decision: Decision,
    pub evidence_refs: Vec<String>,
    pub missing: Vec<String>,
    pub next_evidence: Vec<String>,
}

pub fn evaluate_rules(store: &EvidenceStore, rules: &[Rule]) -> Vec<RuleEvaluation> {
    rules
        .iter()
        .map(|rule| evaluate_rule(store, rule))
        .collect()
}

pub fn claim_for_evaluation(evaluation: &RuleEvaluation) -> Claim {
    let mut next_evidence = claim_definition(&evaluation.claim_id)
        .map(|definition| {
            definition
                .default_next_evidence
                .iter()
                .map(|item| (*item).to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    next_evidence.extend(evaluation.next_evidence.clone());
    next_evidence.sort();
    next_evidence.dedup();
    Claim {
        claim_id: evaluation.claim_id.clone(),
        decision: evaluation.decision.clone(),
        evidence_refs: evaluation.evidence_refs.clone(),
        next_evidence,
    }
}

fn evaluate_rule(store: &EvidenceStore, rule: &Rule) -> RuleEvaluation {
    let matched = eval_pred(store, &rule.when);
    let evidence_refs = refs_for_kinds(store, rule.evidence_kinds);
    let missing = rule
        .evidence_kinds
        .iter()
        .filter(|kind| store.iter(**kind).next().is_none())
        .map(kind_label)
        .collect::<Vec<_>>();
    let next_evidence = if matched {
        Vec::new()
    } else {
        rule.next_evidence
            .iter()
            .map(|item| (*item).to_string())
            .collect()
    };
    RuleEvaluation {
        rule_id: rule.id.to_string(),
        claim_id: rule.claim_id.to_string(),
        matched,
        decision: if matched {
            rule.on_match.clone()
        } else {
            rule.on_miss.clone()
        },
        evidence_refs,
        missing,
        next_evidence,
    }
}

fn eval_pred(store: &EvidenceStore, pred: &Pred) -> bool {
    match pred {
        Pred::Present(kind) => store.iter(*kind).next().is_some(),
        Pred::PressureEffect(pressure_kind) => pressure_effect_observed(store, pressure_kind),
        Pred::CompositeMeasured(scenario) => composite_measured(store, scenario),
        Pred::LoadDurationAtLeastSeconds(seconds) => {
            load_duration_at_least_seconds(store, *seconds)
        }
        Pred::NetworkBoundedTransfer => network_bounded_transfer(store),
        Pred::ObservationSamplesAtLeast(samples) => observation_samples_at_least(store, *samples),
        Pred::RunValidationMeasured => run_validation_measured(store),
        Pred::All(preds) => preds.iter().all(|pred| eval_pred(store, pred)),
        Pred::Any(preds) => preds.iter().any(|pred| eval_pred(store, pred)),
        Pred::Not(pred) => !eval_pred(store, pred),
        Pred::Custom(_, func) => func(store),
    }
}

fn run_validation_measured(store: &EvidenceStore) -> bool {
    let mut saw_fullset_validation = false;
    let mut all_fullset_validations_measured = true;
    for meta in store.iter(Kind::ReportRunValidation) {
        let Ok(artifact) = store.load::<RunValidationPayload>(meta) else {
            all_fullset_validations_measured = false;
            continue;
        };
        if artifact.payload.profile != FULLSET_PROFILE {
            continue;
        }
        saw_fullset_validation = true;
        all_fullset_validations_measured &= is_measured_fullset_validation(&artifact);
    }
    saw_fullset_validation && all_fullset_validations_measured
}

fn pressure_effect_observed(store: &EvidenceStore, pressure_kind: &str) -> bool {
    store.iter(Kind::Pressure).any(|meta| {
        store.load::<PressurePayload>(meta).is_ok_and(|artifact| {
            is_measured_status(&artifact.status)
                && artifact.payload.pressure_kind == pressure_kind
                && artifact.payload.effect_observed
        })
    })
}

fn composite_measured(store: &EvidenceStore, scenario: &str) -> bool {
    store.iter(Kind::Composite).any(|meta| {
        store.load::<CompositePayload>(meta).is_ok_and(|artifact| {
            is_measured_status(&artifact.status)
                && artifact.payload.scenario == scenario
                && artifact.payload.coupling_evidence_class == "composite_measured"
        })
    })
}

fn load_duration_at_least_seconds(store: &EvidenceStore, seconds: u64) -> bool {
    let minimum_ms = seconds.saturating_mul(1000);
    store.iter(Kind::Load).any(|meta| {
        store.load::<LoadPayload>(meta).is_ok_and(|artifact| {
            is_measured_status(&artifact.status) && artifact.payload.duration_ms >= minimum_ms
        })
    })
}

fn network_bounded_transfer(store: &EvidenceStore) -> bool {
    store.iter(Kind::Pressure).any(|meta| {
        store.load::<PressurePayload>(meta).is_ok_and(|artifact| {
            is_measured_status(&artifact.status)
                && artifact.payload.pressure_kind == "network_io"
                && artifact.payload.network_mode.as_deref() == Some("bounded_transfer")
                && artifact.payload.network_endpoint_available == Some(true)
                && artifact
                    .payload
                    .network_traffic_generated_bytes
                    .is_some_and(|bytes| bytes > 0)
        })
    })
}

fn observation_samples_at_least(store: &EvidenceStore, samples: usize) -> bool {
    store.iter(Kind::Observation).any(|meta| {
        store
            .load::<ObservationPayload>(meta)
            .is_ok_and(|artifact| {
                is_measured_status(&artifact.status) && artifact.payload.sample_count >= samples
            })
    })
}

fn is_measured_status(status: &Status) -> bool {
    matches!(status, Status::Measured | Status::MeasuredPartial)
}

fn refs_for_kinds(store: &EvidenceStore, kinds: &[Kind]) -> Vec<String> {
    let mut refs = kinds
        .iter()
        .flat_map(|kind| store.iter(*kind))
        .map(artifact_ref)
        .collect::<Vec<_>>();
    refs.sort();
    refs.dedup();
    refs
}

fn artifact_ref(meta: &ArtifactMeta) -> String {
    meta.artifact_ref.clone()
}

fn kind_label(kind: &Kind) -> String {
    serde_json::to_string(kind)
        .unwrap_or_else(|_| format!("{kind:?}"))
        .trim_matches('"')
        .to_string()
}
