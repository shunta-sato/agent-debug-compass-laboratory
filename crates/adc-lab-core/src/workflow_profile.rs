use crate::{LabError, LabResult};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const WORKFLOW_PROFILE_LEGACY_FULLSET: &str = "target-operating-contract-fullset";
pub const WORKFLOW_PROFILE_SMOKE: &str = "target-operating-contract-smoke";
pub const WORKFLOW_PROFILE_CHARACTERIZATION_FULL: &str = "target-characterization-full";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowProfileDepth {
    Smoke,
    CharacterizationFull,
}

impl WorkflowProfileDepth {
    pub fn parse(value: &str) -> LabResult<Self> {
        match value {
            "smoke" => Ok(Self::Smoke),
            "characterization-full" => Ok(Self::CharacterizationFull),
            _ => Err(LabError::Validation(format!(
                "unsupported profile depth {value}; expected smoke or characterization-full"
            ))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::CharacterizationFull => "characterization-full",
        }
    }

    pub fn effective_profile(self) -> &'static str {
        match self {
            Self::Smoke => WORKFLOW_PROFILE_SMOKE,
            Self::CharacterizationFull => WORKFLOW_PROFILE_CHARACTERIZATION_FULL,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowProfile {
    pub requested_profile: String,
    pub effective_profile: String,
    pub depth: WorkflowProfileDepth,
    pub summary: &'static str,
    pub claim_boundary: &'static str,
    pub coverage: &'static str,
    pub safety_caps: &'static str,
}

pub fn resolve_workflow_profile(
    requested_profile: &str,
    explicit_depth: Option<WorkflowProfileDepth>,
) -> LabResult<WorkflowProfile> {
    let depth = match requested_profile {
        WORKFLOW_PROFILE_SMOKE => require_matching_depth(
            requested_profile,
            explicit_depth,
            WorkflowProfileDepth::Smoke,
        )?,
        WORKFLOW_PROFILE_CHARACTERIZATION_FULL => require_matching_depth(
            requested_profile,
            explicit_depth,
            WorkflowProfileDepth::CharacterizationFull,
        )?,
        WORKFLOW_PROFILE_LEGACY_FULLSET => explicit_depth.ok_or_else(|| {
            LabError::Validation(
                "legacy target-operating-contract-fullset requires --profile-depth smoke or --profile-depth characterization-full; prefer target-operating-contract-smoke or target-characterization-full".to_string(),
            )
        })?,
        _ => {
            return Err(LabError::Validation(format!(
                "unsupported workflow profile {requested_profile}; expected {WORKFLOW_PROFILE_SMOKE}, {WORKFLOW_PROFILE_CHARACTERIZATION_FULL}, or legacy {WORKFLOW_PROFILE_LEGACY_FULLSET} with --profile-depth"
            )));
        }
    };
    Ok(profile_for_depth(requested_profile.to_string(), depth))
}

pub fn supported_validation_profile(profile: &str) -> bool {
    matches!(
        profile,
        WORKFLOW_PROFILE_LEGACY_FULLSET
            | WORKFLOW_PROFILE_SMOKE
            | WORKFLOW_PROFILE_CHARACTERIZATION_FULL
    )
}

pub fn measured_validation_profile(profile: &str) -> bool {
    matches!(
        profile,
        WORKFLOW_PROFILE_SMOKE | WORKFLOW_PROFILE_CHARACTERIZATION_FULL
    )
}

fn require_matching_depth(
    requested_profile: &str,
    explicit_depth: Option<WorkflowProfileDepth>,
    expected: WorkflowProfileDepth,
) -> LabResult<WorkflowProfileDepth> {
    if let Some(actual) = explicit_depth {
        if actual != expected {
            return Err(LabError::Validation(format!(
                "{requested_profile} requires --profile-depth {}",
                expected.as_str()
            )));
        }
    }
    Ok(expected)
}

fn profile_for_depth(requested_profile: String, depth: WorkflowProfileDepth) -> WorkflowProfile {
    match depth {
        WorkflowProfileDepth::Smoke => WorkflowProfile {
            requested_profile,
            effective_profile: WORKFLOW_PROFILE_SMOKE.to_string(),
            depth,
            summary: "smoke profile: workflow correctness, runner preflight, read-only identity, short seed probes, target-local workload demand, and governor-validation smoke",
            claim_boundary: "smoke evidence is not deep target characterization and must not support production, selection, 24h safety, or Pi4/Pi5 sufficiency claims",
            coverage: "coverage: preflight, inventory, toolchain, 30s observe baseline, 10s CPU seed, bounded pressure/composite smoke, target-local workload demand, governor sweep smoke",
            safety_caps: "safety caps: short bounded durations, explicit worker counts, operator approval for governor sweep, restore-after-each, thermal abort where control paths support it",
        },
        WorkflowProfileDepth::CharacterizationFull => WorkflowProfile {
            requested_profile,
            effective_profile: WORKFLOW_PROFILE_CHARACTERIZATION_FULL.to_string(),
            depth,
            summary: "characterization-full profile: CPU/thermal characterization plus pressure/composite coverage with explicit duration, worker, thermal abort, cooldown, network endpoint, and 300s-not-24h boundaries; suitability linkage remains staged for later v0.2.4 PRs",
            claim_boundary: "characterization-full collect plans are not production, 24h safety, battery safety, target selection, or real workload performance evidence; pressure presence alone does not prove coupling, and counter-only network evidence does not prove bounded transfer",
            coverage: "coverage in PR5: 60s and 300s passive observations, 1/2/4 worker 60s CPU ladder, three 4-worker 60s repeatability trials with cooldown, 4-worker 300s sustained bounded load, cooldown observation, bounded pressure map for latency_jitter/observer_pressure/memory_pressure/storage_io/cpu_pressure/thermal_pressure/network_io, endpoint-backed network transfer when --network-endpoint is supplied, and memory_storage_jitter composite probe",
            safety_caps: "safety caps: 75C thermal abort by default, explicit durations and worker counts, cooldown observations, approval-bound governor control, restore validation, no arbitrary root shell; optional approved 900s profile disabled by default",
        },
    }
}
