use schemars::JsonSchema;
use serde::Serialize;

pub mod claim {
    pub const TARGET_SELECTION_PI4_SUFFICIENT: &str = "target.selection.pi4_sufficient";
    pub const PRODUCTION_READY: &str = "target.selection.production_ready";
    pub const COUPLING_MEMORY_TO_STORAGE: &str = "coupling.memory_to_storage";
    pub const BATTERY_SAFE: &str = "target.selection.battery_safe";
    pub const REAL_TIME_PRESSURE_SAFE: &str = "target.selection.real_time_pressure_safe";
    pub const SELECTION_READY: &str = "target.selection.selection_ready";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimDefinition {
    pub claim_id: &'static str,
    pub supported_text: &'static str,
    pub blocked_text: &'static str,
    pub blocked_claim: &'static str,
    pub default_next_evidence: &'static [&'static str],
}

const CLAIMS: &[ClaimDefinition] = &[
    ClaimDefinition {
        claim_id: claim::TARGET_SELECTION_PI4_SUFFICIENT,
        supported_text: "Pi4 is sufficient for the stated workload under the recorded policy and evidence.",
        blocked_text: "Do not claim Pi4 sufficiency without workload demand, target evidence, and suitability policy margins.",
        blocked_claim: "Pi4 is sufficient",
        default_next_evidence: &[
            "collect workload demand profile",
            "evaluate suitability policy against target evidence",
        ],
    },
    ClaimDefinition {
        claim_id: claim::PRODUCTION_READY,
        supported_text: "The target is production-ready for the stated workload and operating envelope.",
        blocked_text: "Do not claim production readiness from exploratory lab evidence.",
        blocked_claim: "production readiness",
        default_next_evidence: &[
            "define production operating envelope",
            "run controlled long-duration validation",
            "record recovery and degradation behavior",
        ],
    },
    ClaimDefinition {
        claim_id: claim::COUPLING_MEMORY_TO_STORAGE,
        supported_text: "Memory pressure coupling to storage behavior is measured for this scenario.",
        blocked_text: "Do not claim memory-to-storage coupling without paired pressure and recovery evidence.",
        blocked_claim: "memory-to-storage coupling",
        default_next_evidence: &[
            "run baseline, pressure-only, paired-pressure, and recovery phases",
            "record pressure effect and side effects",
        ],
    },
    ClaimDefinition {
        claim_id: claim::BATTERY_SAFE,
        supported_text: "Battery impact is bounded for the stated workload and target mode.",
        blocked_text: "Do not claim battery safety without target-local power evidence.",
        blocked_claim: "battery safe",
        default_next_evidence: &[
            "collect target-local power or battery discharge evidence",
            "compare workload and idle power over the same target mode",
        ],
    },
    ClaimDefinition {
        claim_id: claim::REAL_TIME_PRESSURE_SAFE,
        supported_text: "Latency behavior is bounded under the relevant pressure conditions.",
        blocked_text: "Do not claim real-time safety under pressure without pressure-specific jitter evidence.",
        blocked_claim: "real-time safe under all pressure",
        default_next_evidence: &[
            "run pressure-specific jitter probes",
            "record p95, p99, max latency, and recovery behavior",
        ],
    },
    ClaimDefinition {
        claim_id: claim::SELECTION_READY,
        supported_text: "The target/workload pair is ready for selection under the stated evidence policy.",
        blocked_text: "Do not claim selection readiness while required suitability dimensions are unknown or failed.",
        blocked_claim: "selection readiness",
        default_next_evidence: &[
            "resolve unknown required suitability dimensions",
            "rerun suitability policy after missing evidence is collected",
        ],
    },
];

pub fn all_claims() -> &'static [ClaimDefinition] {
    CLAIMS
}

pub fn claim_definition(claim_id: &str) -> Option<&'static ClaimDefinition> {
    CLAIMS
        .iter()
        .find(|definition| definition.claim_id == claim_id)
}

pub fn blocked_claims_for(claim_ids: &[&str]) -> Vec<String> {
    let mut claims = claim_ids
        .iter()
        .filter_map(|claim_id| claim_definition(claim_id))
        .map(|definition| definition.blocked_claim.to_string())
        .collect::<Vec<_>>();
    claims.sort();
    claims.dedup();
    claims
}
