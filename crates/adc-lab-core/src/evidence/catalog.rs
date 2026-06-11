use schemars::JsonSchema;
use serde::Serialize;

pub mod claim {
    pub const TARGET_SELECTION_PI4_SUFFICIENT: &str = "target.selection.pi4_sufficient";
    pub const PRODUCTION_READY: &str = "target.selection.production_ready";
    pub const COUPLING_MEMORY_TO_STORAGE: &str = "coupling.memory_to_storage";
    pub const THERMAL_SUSTAINED_SOAK: &str = "boundary.thermal_sustained_soak";
    pub const STORAGE_DEFAULT_WRITES_BOUNDED: &str = "boundary.storage_default_writes_bounded";
    pub const NETWORK_BOUNDED_TRANSFER: &str = "boundary.network_bounded_transfer";
    pub const OBSERVER_CADENCE_BOUNDED: &str = "boundary.observer_cadence_bounded";
    pub const BATTERY_SAFE: &str = "target.selection.battery_safe";
    pub const REAL_TIME_PRESSURE_SAFE: &str = "target.selection.real_time_pressure_safe";
    pub const SELECTION_READY: &str = "target.selection.selection_ready";
    pub const RUN_TARGET_INVENTORY_COLLECTED: &str = "run.target_inventory_collected";
    pub const RUN_TOOLCHAIN_INVENTORY_COLLECTED: &str = "run.toolchain_inventory_collected";
    pub const RUN_TOOL_QUALIFICATION_SUMMARY_GENERATED: &str =
        "run.tool_qualification_summary_generated";
    pub const RUN_PASSIVE_OBSERVATION_COLLECTED: &str = "run.passive_observation_collected";
    pub const RUN_BOUNDED_LOAD_COMPLETED: &str = "run.bounded_load_completed";
    pub const RUN_EXPERIMENT_BOUNDED_MATRIX_EXECUTED: &str =
        "run.experiment_bounded_matrix_executed";
    pub const OPERATING_POINT_BOUNDED_WORKLOAD_MEASURED: &str =
        "operating_point.bounded_workload_measured";
    pub const OPERATING_POINT_FIXED_CPU_FREQUENCY_VERIFIED: &str =
        "operating_point.fixed_cpu_frequency_verified";
    pub const OPERATING_POINT_ALL_POINTS_MEASURED: &str = "operating_point.all_points_measured";
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
        claim_id: claim::THERMAL_SUSTAINED_SOAK,
        supported_text: "Thermal behavior is bounded for a sustained approved soak window.",
        blocked_text: "Do not claim sustained thermal margin from short or unpaired load evidence.",
        blocked_claim: "sustained thermal margin",
        default_next_evidence: &[
            "run approved sustained thermal soak",
            "record temperature, frequency, abort, and cooldown behavior",
        ],
    },
    ClaimDefinition {
        claim_id: claim::STORAGE_DEFAULT_WRITES_BOUNDED,
        supported_text: "Default storage writes are bounded for the recorded probe path.",
        blocked_text: "Do not claim safe default storage cadence without bounded write and cleanup evidence.",
        blocked_claim: "bounded default storage writes",
        default_next_evidence: &[
            "run bounded storage I/O probe",
            "record bytes, latency, cleanup, and side effects",
        ],
    },
    ClaimDefinition {
        claim_id: claim::NETWORK_BOUNDED_TRANSFER,
        supported_text: "Network I/O evidence includes an endpoint-backed bounded transfer.",
        blocked_text: "Do not claim network I/O boundaries from counters or endpoint attempts alone.",
        blocked_claim: "bounded network transfer",
        default_next_evidence: &[
            "run endpoint-backed bounded transfer",
            "record generated bytes, endpoint availability, and latency side effects",
        ],
    },
    ClaimDefinition {
        claim_id: claim::OBSERVER_CADENCE_BOUNDED,
        supported_text: "Observer cadence is bounded for the recorded observation path.",
        blocked_text: "Do not claim low-overhead observer behavior without cadence and artifact evidence.",
        blocked_claim: "bounded observer cadence",
        default_next_evidence: &[
            "record observation sample cadence",
            "run observer pressure or observer-off/on workload comparison",
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
    ClaimDefinition {
        claim_id: claim::RUN_TARGET_INVENTORY_COLLECTED,
        supported_text: "Target inventory was collected through read-only surfaces.",
        blocked_text: "Do not claim target identity or hardware capability without read-only target inventory.",
        blocked_claim: "target inventory collected",
        default_next_evidence: &["collect read-only target inventory"],
    },
    ClaimDefinition {
        claim_id: claim::RUN_TOOLCHAIN_INVENTORY_COLLECTED,
        supported_text: "Toolchain inventory was collected through read-only discovery.",
        blocked_text: "Do not claim tool availability without read-only toolchain inventory.",
        blocked_claim: "toolchain inventory collected",
        default_next_evidence: &["collect read-only toolchain inventory"],
    },
    ClaimDefinition {
        claim_id: claim::RUN_TOOL_QUALIFICATION_SUMMARY_GENERATED,
        supported_text: "Tool qualification summary was generated for discovered tools.",
        blocked_text: "Do not treat tool output as qualified evidence without a qualification summary.",
        blocked_claim: "tool qualification summary generated",
        default_next_evidence: &["generate tool qualification summary"],
    },
    ClaimDefinition {
        claim_id: claim::RUN_PASSIVE_OBSERVATION_COLLECTED,
        supported_text: "Passive resource signals were sampled under the current target policy.",
        blocked_text: "Do not claim passive resource behavior without observation evidence.",
        blocked_claim: "passive observation collected",
        default_next_evidence: &[
            "collect passive observation",
            "run controlled operating point matrix before stronger claims",
        ],
    },
    ClaimDefinition {
        claim_id: claim::RUN_BOUNDED_LOAD_COMPLETED,
        supported_text: "Bounded CPU load completed under configured safety bounds.",
        blocked_text: "Do not claim bounded workload completion without a load result artifact.",
        blocked_claim: "bounded workload completion",
        default_next_evidence: &["run bounded CPU load with safety monitor"],
    },
    ClaimDefinition {
        claim_id: claim::RUN_EXPERIMENT_BOUNDED_MATRIX_EXECUTED,
        supported_text: "A bounded non-privileged experiment matrix completed at least one trial.",
        blocked_text: "Do not claim experiment execution when all trials are planned, blocked, or failed.",
        blocked_claim: "bounded experiment matrix executed",
        default_next_evidence: &["execute a supported cpu_load_workers matrix"],
    },
    ClaimDefinition {
        claim_id: claim::OPERATING_POINT_BOUNDED_WORKLOAD_MEASURED,
        supported_text: "Bounded workload operating points were measured for completed trials.",
        blocked_text: "Do not claim measured workload operating points without completed trial evidence.",
        blocked_claim: "bounded workload operating points measured",
        default_next_evidence: &["execute a supported controlled workload matrix"],
    },
    ClaimDefinition {
        claim_id: claim::OPERATING_POINT_FIXED_CPU_FREQUENCY_VERIFIED,
        supported_text: "Fixed CPU frequency behavior was verified under controlled conditions.",
        blocked_text: "Do not claim fixed CPU frequency behavior from passive frequency variation.",
        blocked_claim: "fixed CPU frequency behavior",
        default_next_evidence: &[
            "approved privileged control plan",
            "controlled fixed-frequency matrix",
            "restore verification per point",
        ],
    },
    ClaimDefinition {
        claim_id: claim::OPERATING_POINT_ALL_POINTS_MEASURED,
        supported_text: "All required operating points were measured with controlled factors.",
        blocked_text: "Do not claim all operating points were measured from observational or subset evidence.",
        blocked_claim: "all operating points measured",
        default_next_evidence: &["controlled operating point matrix across required factors"],
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
