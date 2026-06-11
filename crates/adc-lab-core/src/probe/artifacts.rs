use crate::contracts::{
    CompositeBoundaryResult, ContractEvidenceStatus, LoadRestoreOnAbortStatus, LoadResult,
    NetworkPressureMode, ResourceCouplingEvidenceClass, ResourcePressureEvidenceClass,
    ResourcePressureResult, WorkloadDemandProfile,
};
use crate::evidence::{
    Artifact, Bounds, DataQuality, DataQualityLevel, EvidenceStore, Factors, Kind, Metric, Status,
};
use crate::ids::{new_id, now_unix_ms};
use crate::observe::ObservationResult;
use crate::run::run_id_from_run_dir;
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
    pub abort_reason: Option<String>,
    pub workers: usize,
    pub duration_ms: u64,
    pub max_observed_temp_c: Option<f64>,
    pub operator_abort_observed: bool,
    pub safety_monitor_samples: u64,
    pub thermal_surface_available: bool,
    pub restore_on_abort_status: String,
    pub worker_iterations: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PressurePayload {
    pub source_schema_version: String,
    pub pressure_kind: String,
    pub evidence_class: String,
    pub effect_observed: bool,
    pub duration_ms: u64,
    pub network_mode: Option<String>,
    pub network_endpoint_available: Option<bool>,
    pub network_traffic_generated_bytes: Option<u64>,
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
            abort_reason: result.abort_reason,
            workers: result.workers,
            duration_ms: result.duration_ms,
            max_observed_temp_c: result.max_observed_temp_c,
            operator_abort_observed: result.safety_monitor.operator_abort_observed,
            safety_monitor_samples: result.safety_monitor.samples,
            thermal_surface_available: result.safety_monitor.thermal_surface_available,
            restore_on_abort_status: load_restore_status_label(
                &result.safety_monitor.restore_on_abort_status,
            )
            .to_string(),
            worker_iterations: result.worker_iterations,
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
            evidence_class: pressure_evidence_class_label(&result.evidence_class).to_string(),
            effect_observed: result.pressure_effect.observed,
            duration_ms: result.duration_ms,
            network_mode: result
                .network_evidence
                .as_ref()
                .map(|evidence| network_mode_label(&evidence.network_mode).to_string()),
            network_endpoint_available: result
                .network_evidence
                .as_ref()
                .map(|evidence| evidence.endpoint_available),
            network_traffic_generated_bytes: result
                .network_evidence
                .as_ref()
                .map(|evidence| evidence.traffic_generated_bytes),
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
            coupling_evidence_class: coupling_evidence_class_label(&result.coupling_evidence_class)
                .to_string(),
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
    let artifact = observation_artifact_v2(run_id_from_run_dir(run_dir), result);
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
    let relative = format!("load/cpu.{}.v2.json", safe_file_segment(&result.result_id));
    let artifact = load_artifact_v2(run_id_from_run_dir(run_dir), result);
    store.write(run_dir, Path::new(&relative), &artifact)
}

pub fn write_pressure_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: ResourcePressureResult,
) -> LabResult<String> {
    let relative = format!(
        "pressure/{}.{}.v2.json",
        result.pressure_kind.as_str(),
        safe_file_segment(&result.result_id)
    );
    let artifact = pressure_artifact_v2(run_id_from_run_dir(run_dir), result);
    store.write(run_dir, Path::new(&relative), &artifact)
}

pub fn write_composite_artifact_v2(
    store: &mut EvidenceStore,
    run_dir: &Path,
    result: CompositeBoundaryResult,
) -> LabResult<String> {
    let relative = format!(
        "composite/{}.{}.v2.json",
        result.scenario.as_str(),
        safe_file_segment(&result.result_id)
    );
    let artifact = composite_artifact_v2(run_id_from_run_dir(run_dir), result);
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

fn network_mode_label(mode: &NetworkPressureMode) -> &'static str {
    match mode {
        NetworkPressureMode::CounterOnly => "counter_only",
        NetworkPressureMode::EndpointAttempt => "endpoint_attempt",
        NetworkPressureMode::BoundedTransfer => "bounded_transfer",
    }
}

fn load_restore_status_label(status: &LoadRestoreOnAbortStatus) -> &'static str {
    match status {
        LoadRestoreOnAbortStatus::NotRequired => "not_required",
        LoadRestoreOnAbortStatus::NotConfigured => "not_configured",
        LoadRestoreOnAbortStatus::Attempted => "attempted",
        LoadRestoreOnAbortStatus::Succeeded => "succeeded",
        LoadRestoreOnAbortStatus::Failed => "failed",
    }
}

fn pressure_evidence_class_label(class: &ResourcePressureEvidenceClass) -> &'static str {
    match class {
        ResourcePressureEvidenceClass::Smoke => "smoke",
        ResourcePressureEvidenceClass::PressureInduced => "pressure_induced",
        ResourcePressureEvidenceClass::PairedPressure => "paired_pressure",
        ResourcePressureEvidenceClass::BoundaryProbe => "boundary_probe",
        ResourcePressureEvidenceClass::NotApplicable => "not_applicable",
    }
}

fn coupling_evidence_class_label(class: &ResourceCouplingEvidenceClass) -> &'static str {
    match class {
        ResourceCouplingEvidenceClass::IngredientsOnly => "ingredients_only",
        ResourceCouplingEvidenceClass::CompositeMeasured => "composite_measured",
        ResourceCouplingEvidenceClass::CouplingNotMeasured => "coupling_not_measured",
    }
}

fn safe_file_segment(value: &str) -> String {
    let segment = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if segment.is_empty() {
        "unknown".to_string()
    } else {
        segment
    }
}
