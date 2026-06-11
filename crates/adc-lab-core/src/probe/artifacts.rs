use crate::contracts::{
    CompositeBoundaryResult, ContractEvidenceStatus, LoadResult, ResourcePressureResult,
    WorkloadDemandProfile,
};
use crate::evidence::{
    Artifact, Bounds, DataQuality, DataQualityLevel, EvidenceStore, Factors, Kind, Metric, Status,
};
use crate::ids::{new_id, now_unix_ms};
use crate::observe::ObservationResult;
use crate::LabResult;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ObservationPayload {
    pub source_schema_version: String,
    pub duration_ms: u64,
    pub sample_count: usize,
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LoadPayload {
    pub source_schema_version: String,
    pub load_id: String,
    pub status: String,
    pub workers: usize,
    pub duration_ms: u64,
    pub max_observed_temp_c: Option<f64>,
    pub operator_abort_observed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PressurePayload {
    pub source_schema_version: String,
    pub pressure_kind: String,
    pub evidence_class: String,
    pub effect_observed: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompositePayload {
    pub source_schema_version: String,
    pub scenario: String,
    pub coupling_evidence_class: String,
    pub phase_count: usize,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkloadPayload {
    pub source_schema_version: String,
    pub workload_id: String,
    pub execution_mode: String,
    pub demand_scope: String,
    pub degraded_data_quality: bool,
}

pub fn observation_artifact_v2(
    run_id: impl Into<String>,
    result: ObservationResult,
) -> Artifact<ObservationPayload> {
    let mut artifact = Artifact::new(
        Kind::Observation,
        new_id("OBSERVATION"),
        run_id,
        result.target_id,
        Status::Measured,
        ObservationPayload {
            source_schema_version: result.schema_version,
            duration_ms: result.duration_ms,
            sample_count: result.samples.len(),
            signals: result
                .signals
                .iter()
                .map(|signal| format!("{signal:?}").to_ascii_lowercase())
                .collect(),
        },
        now_unix_ms(),
    );
    artifact.bounds = Some(Bounds {
        duration_seconds_max: Some(artifact.payload.duration_ms.div_ceil(1000)),
        notes: vec!["wrapped from v1 observation result".to_string()],
    });
    artifact
}

pub fn load_artifact_v2(run_id: impl Into<String>, result: LoadResult) -> Artifact<LoadPayload> {
    let status = if result.status == "completed" {
        Status::Measured
    } else if result.abort_reason.is_some() {
        Status::MeasuredPartial
    } else {
        Status::Insufficient
    };
    let mut artifact = Artifact::new(
        Kind::Load,
        result.result_id.clone(),
        run_id,
        result.target_id,
        status,
        LoadPayload {
            source_schema_version: result.schema_version,
            load_id: result.load_id,
            status: result.status,
            workers: result.workers,
            duration_ms: result.duration_ms,
            max_observed_temp_c: result.max_observed_temp_c,
            operator_abort_observed: result.safety_monitor.operator_abort_observed,
        },
        now_unix_ms(),
    );
    artifact.bounds = Some(Bounds {
        duration_seconds_max: Some(artifact.payload.duration_ms.div_ceil(1000)),
        notes: vec!["wrapped from v1 load result".to_string()],
    });
    artifact
}

pub fn pressure_artifact_v2(
    run_id: impl Into<String>,
    result: ResourcePressureResult,
) -> Artifact<PressurePayload> {
    let mut artifact = Artifact::new(
        Kind::Pressure,
        result.result_id.clone(),
        run_id,
        result.target_id,
        status_from_contract(&result.status),
        PressurePayload {
            source_schema_version: result.schema_version,
            pressure_kind: result.pressure_kind.as_str().to_string(),
            evidence_class: format!("{:?}", result.evidence_class).to_ascii_lowercase(),
            effect_observed: result.pressure_effect.observed,
            duration_ms: result.duration_ms,
        },
        now_unix_ms(),
    );
    artifact.bounds = Some(Bounds {
        duration_seconds_max: Some(result.safety.duration_seconds_max),
        notes: result.safety.abort_conditions,
    });
    artifact.factors = Factors {
        controlled: result
            .controlled_factors
            .iter()
            .map(|factor| format!("{}={}", factor.factor_id, factor.value))
            .collect(),
        observed: result
            .observed_covariates
            .iter()
            .map(|factor| format!("{}={}", factor.factor_id, factor.value))
            .collect(),
        confounders: result.uncontrolled_confounders,
    };
    artifact.metrics = result
        .metrics
        .iter()
        .filter_map(|metric| {
            metric.value.map(|value| Metric {
                name: metric.metric_id.clone(),
                value,
                unit: Some(metric.unit.clone()),
            })
        })
        .collect();
    artifact.evidence_refs = result.evidence_refs;
    artifact
}

pub fn composite_artifact_v2(
    run_id: impl Into<String>,
    result: CompositeBoundaryResult,
) -> Artifact<CompositePayload> {
    let mut artifact = Artifact::new(
        Kind::Composite,
        result.result_id.clone(),
        run_id,
        result.target_id,
        status_from_contract(&result.status),
        CompositePayload {
            source_schema_version: result.schema_version,
            scenario: result.scenario.as_str().to_string(),
            coupling_evidence_class: format!("{:?}", result.coupling_evidence_class)
                .to_ascii_lowercase(),
            phase_count: result.phases.len(),
            duration_ms: result.duration_ms,
        },
        now_unix_ms(),
    );
    artifact.bounds = Some(Bounds {
        duration_seconds_max: Some(result.safety.duration_seconds_max),
        notes: result.safety.abort_conditions,
    });
    artifact.factors = Factors {
        controlled: result
            .controlled_factors
            .iter()
            .map(|factor| format!("{}={}", factor.factor_id, factor.value))
            .collect(),
        observed: result
            .observed_covariates
            .iter()
            .map(|factor| format!("{}={}", factor.factor_id, factor.value))
            .collect(),
        confounders: result.uncontrolled_confounders,
    };
    artifact.evidence_refs = result.evidence_refs;
    artifact
}

pub fn workload_artifact_v2(profile: WorkloadDemandProfile) -> Artifact<WorkloadPayload> {
    let status = if profile.data_quality.degraded {
        Status::MeasuredPartial
    } else {
        Status::Measured
    };
    let mut artifact = Artifact::new(
        Kind::Workload,
        profile.profile_id.clone(),
        profile.run_id,
        profile.target_id,
        status,
        WorkloadPayload {
            source_schema_version: profile.schema_version,
            workload_id: profile.workload_id,
            execution_mode: format!("{:?}", profile.execution_mode).to_ascii_lowercase(),
            demand_scope: format!("{:?}", profile.demand_scope).to_ascii_lowercase(),
            degraded_data_quality: profile.data_quality.degraded,
        },
        now_unix_ms(),
    );
    artifact.data_quality = DataQuality {
        level: if artifact.payload.degraded_data_quality {
            DataQualityLevel::Degraded
        } else {
            DataQualityLevel::Complete
        },
        notes: profile.data_quality.notes,
    };
    artifact.evidence_refs = profile.evidence_refs;
    artifact
}

pub fn write_observation_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: ObservationResult,
) -> LabResult<String> {
    let artifact = observation_artifact_v2(run_id_from_dir(run_dir), result);
    store.write(
        run_dir,
        Path::new("observations/observe.v2.json"),
        &artifact,
    )
}

pub fn write_load_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: LoadResult,
) -> LabResult<String> {
    let artifact = load_artifact_v2(run_id_from_dir(run_dir), result);
    store.write(run_dir, Path::new("load/cpu.v2.json"), &artifact)
}

pub fn write_pressure_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: ResourcePressureResult,
) -> LabResult<String> {
    let relative = format!("pressure/{}.v2.json", result.pressure_kind.as_str());
    let artifact = pressure_artifact_v2(run_id_from_dir(run_dir), result);
    store.write(run_dir, Path::new(&relative), &artifact)
}

pub fn write_composite_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: CompositeBoundaryResult,
) -> LabResult<String> {
    let relative = format!("composite/{}.v2.json", result.scenario.as_str());
    let artifact = composite_artifact_v2(run_id_from_dir(run_dir), result);
    store.write(run_dir, Path::new(&relative), &artifact)
}

pub fn write_workload_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    profile: WorkloadDemandProfile,
) -> LabResult<String> {
    let artifact = workload_artifact_v2(profile);
    store.write(
        run_dir,
        Path::new("workload/demand_profile.v2.json"),
        &artifact,
    )
}

fn status_from_contract(status: &ContractEvidenceStatus) -> Status {
    match status {
        ContractEvidenceStatus::Measured => Status::Measured,
        ContractEvidenceStatus::MeasuredPartial => Status::MeasuredPartial,
        ContractEvidenceStatus::NotApplicableWithReason => Status::NotApplicable {
            reason: "v1 contract marked not applicable".to_string(),
        },
        ContractEvidenceStatus::UnsafeToRunWithReason => Status::UnsafeBlocked {
            reason: "v1 contract marked unsafe to run".to_string(),
        },
        ContractEvidenceStatus::NotControllable | ContractEvidenceStatus::Insufficient => {
            Status::Insufficient
        }
    }
}

fn run_id_from_dir(run_dir: &Path) -> String {
    run_dir
        .file_name()
        .and_then(|value| value.to_str())
        .map(|value| value.to_string())
        .unwrap_or_else(|| "LAB-RUN-v2".to_string())
}
