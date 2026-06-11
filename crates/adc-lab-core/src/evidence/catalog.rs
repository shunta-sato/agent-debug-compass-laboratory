use schemars::JsonSchema;
use serde::Serialize;

pub mod claim {
    pub const TARGET_SELECTION_PI4_SUFFICIENT: &str = "target.selection.pi4_sufficient";
    pub const PRODUCTION_READY: &str = "target.selection.production_ready";
    pub const COUPLING_MEMORY_TO_STORAGE: &str = "coupling.memory_to_storage";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimDefinition {
    pub claim_id: &'static str,
    pub supported_text: &'static str,
    pub blocked_text: &'static str,
    pub default_next_evidence: &'static [&'static str],
}

const CLAIMS: &[ClaimDefinition] = &[
    ClaimDefinition {
        claim_id: claim::TARGET_SELECTION_PI4_SUFFICIENT,
        supported_text: "Pi4 is sufficient for the stated workload under the recorded policy and evidence.",
        blocked_text: "Do not claim Pi4 sufficiency without workload demand, target evidence, and suitability policy margins.",
        default_next_evidence: &[
            "collect workload demand profile",
            "evaluate suitability policy against target evidence",
        ],
    },
    ClaimDefinition {
        claim_id: claim::PRODUCTION_READY,
        supported_text: "The target is production-ready for the stated workload and operating envelope.",
        blocked_text: "Do not claim production readiness from exploratory lab evidence.",
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
        default_next_evidence: &[
            "run baseline, pressure-only, paired-pressure, and recovery phases",
            "record pressure effect and side effects",
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
