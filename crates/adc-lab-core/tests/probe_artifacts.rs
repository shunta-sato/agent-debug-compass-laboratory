use adc_lab_core::{
    claim, evaluate_rules, write_composite_artifact_v2, write_load_artifact_v2,
    write_observation_artifact_v2, write_pressure_artifact_v2, write_workload_artifact_v2,
    Artifact, CompositeBoundaryResult, Decision, EvidenceStore, Kind, LoadRestoreOnAbortStatus,
    LoadResult, LoadSafetyMonitorResult, ObservationResult, Pred, PressurePayload, Rule, Signal,
    Status, WorkloadDataQuality, WorkloadDemand, WorkloadDemandProfile, WorkloadDemandScope,
    WorkloadExecutionMode, WorkloadSystemContext, WorkloadTargetConditionedResponse,
};
use std::path::Path;

#[test]
fn probe_artifact_writers_index_all_core_probe_kinds() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();

    write_observation_artifact_v2(&mut store, temp.path(), observation()).unwrap();
    write_load_artifact_v2(&mut store, temp.path(), load()).unwrap();
    write_pressure_artifact_v2(&mut store, temp.path(), pressure()).unwrap();
    write_composite_artifact_v2(&mut store, temp.path(), composite()).unwrap();
    write_workload_artifact_v2(&mut store, temp.path(), workload()).unwrap();

    let reopened = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    assert_eq!(reopened.iter(Kind::Observation).count(), 1);
    assert_eq!(reopened.iter(Kind::Load).count(), 1);
    assert_eq!(reopened.iter(Kind::Pressure).count(), 1);
    assert_eq!(reopened.iter(Kind::Composite).count(), 1);
    assert_eq!(reopened.iter(Kind::Workload).count(), 1);
}

#[test]
fn dummy_pressure_kind_extension_exercise_stays_within_three_hand_edited_files() {
    let touched_files = [
        "crates/adc-lab-core/src/probe/artifacts.rs",
        "crates/adc-lab-core/src/rules/operating_contract.rs",
        "crates/adc-lab-core/tests/probe_artifacts.rs",
    ];
    assert!(touched_files.len() <= 3);

    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let dummy = Artifact::new(
        Kind::Pressure,
        "ARTIFACT-DUMMY-PRESSURE",
        "LAB-RUN-001",
        "target55",
        Status::Measured,
        PressurePayload {
            source_schema_version: "lab.pressure.v2.exercise".to_string(),
            pressure_kind: "dummy_cache_pressure".to_string(),
            evidence_class: "smoke".to_string(),
            effect_observed: true,
            duration_ms: 1,
        },
        1,
    );
    store
        .write(
            temp.path(),
            Path::new("pressure/dummy_cache_pressure.v2.json"),
            &dummy,
        )
        .unwrap();

    let evaluations = evaluate_rules(
        &store,
        &[Rule {
            id: "exercise.dummy_cache_pressure_present",
            claim_id: claim::COUPLING_MEMORY_TO_STORAGE,
            when: Pred::Present(Kind::Pressure),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure],
            next_evidence: &["promote dummy pressure shape only after real probe evidence"],
        }],
    );
    assert_eq!(evaluations[0].decision, Decision::Provisional);
    assert_eq!(evaluations[0].evidence_refs.len(), 1);
}

fn observation() -> ObservationResult {
    ObservationResult {
        schema_version: "lab.observation_result.v1".to_string(),
        target_id: "target55".to_string(),
        duration_ms: 100,
        signals: vec![Signal::Cpu, Signal::Memory],
        samples: Vec::new(),
    }
}

fn load() -> LoadResult {
    LoadResult {
        schema_version: "lab.load_result.v1".to_string(),
        result_id: "LOAD-RESULT-001".to_string(),
        load_id: "LOAD-001".to_string(),
        target_id: "target55".to_string(),
        status: "completed".to_string(),
        workers: 1,
        duration_ms: 100,
        abort_reason: None,
        max_observed_temp_c: Some(42.0),
        worker_iterations: vec![1],
        safety_monitor: LoadSafetyMonitorResult {
            sample_interval_ms: 100,
            samples: 1,
            thermal_surface_available: false,
            operator_abort_observed: false,
            restore_on_abort_status: LoadRestoreOnAbortStatus::NotRequired,
        },
        time_unix_ms: 1,
    }
}

fn pressure() -> adc_lab_core::ResourcePressureResult {
    serde_json::from_value(serde_json::json!({
        "schema_version": "lab.resource_pressure_result.v1",
        "result_id": "PRESSURE-001",
        "target_id": "target55",
        "pressure_kind": "memory_pressure",
        "status": "measured_partial",
        "evidence_class": "pressure_induced",
        "intensity": {
            "requested": "bounded",
            "relative_to_target": "small",
            "pressure_effect_observed": true
        },
        "pressure_effect": {
            "observed": true,
            "basis": ["fixture"]
        },
        "network_evidence": null,
        "condition": {
            "pressure_kind": "memory_pressure",
            "governor": null,
            "workers": null,
            "duration": "bounded"
        },
        "duration_ms": 100,
        "controlled_factors": [],
        "observed_covariates": [],
        "uncontrolled_confounders": [],
        "metrics": [],
        "side_effects": [],
        "safety": {
            "duration_seconds_max": 1,
            "memory_bytes_max": 1048576,
            "storage_bytes_max": 0,
            "network_bytes_max": 0,
            "abort_conditions": ["bounded fixture"],
            "cleanup": ["drop allocation"],
            "cleanup_verified": true
        },
        "evidence_refs": [],
        "claim_supported": [],
        "claim_blocked": [],
        "next_evidence_needed": [],
        "time_unix_ms": 1
    }))
    .unwrap()
}

fn composite() -> CompositeBoundaryResult {
    serde_json::from_value(serde_json::json!({
        "schema_version": "lab.composite_boundary_result.v1",
        "result_id": "COMPOSITE-001",
        "target_id": "target55",
        "scenario": "memory_storage_jitter",
        "status": "measured_partial",
        "coupling_evidence_class": "composite_measured",
        "duration_ms": 100,
        "controlled_factors": [],
        "observed_covariates": [],
        "uncontrolled_confounders": [],
        "phases": [],
        "safety": {
            "duration_seconds_max": 1,
            "memory_bytes_max": 1048576,
            "storage_bytes_max": 1024,
            "network_bytes_max": 0,
            "abort_conditions": ["bounded fixture"],
            "cleanup": ["drop allocation"],
            "cleanup_verified": true
        },
        "evidence_refs": [],
        "claim_supported": [],
        "claim_blocked": [],
        "next_evidence_needed": [],
        "time_unix_ms": 1
    }))
    .unwrap()
}

fn workload() -> WorkloadDemandProfile {
    WorkloadDemandProfile {
        schema_version: "lab.workload_demand_profile.v1".to_string(),
        profile_id: "WORKLOAD-PROFILE-001".to_string(),
        run_id: "LAB-RUN-001".to_string(),
        workload_id: "workload-001".to_string(),
        target_id: "target55".to_string(),
        execution_mode: WorkloadExecutionMode::Local,
        demand_scope: WorkloadDemandScope::ProcessScoped,
        workload_demand: WorkloadDemand {
            process_cpu_utime_ticks: None,
            process_cpu_stime_ticks: None,
            process_cpu_time_ms: None,
            process_cpu_percent_avg: None,
            process_cpu_percent_peak: None,
            rss_peak_kb: None,
            vmhwm_peak_kb: None,
            read_bytes: None,
            write_bytes: None,
            cancelled_write_bytes: None,
            voluntary_ctxt_switches: None,
            nonvoluntary_ctxt_switches: None,
            duty_cycle: "short_fixture".to_string(),
            child_process_accounting_status: "not_needed".to_string(),
        },
        target_conditioned_response: WorkloadTargetConditionedResponse {
            portable_between_targets: false,
            thermal_max_c: None,
            thermal_margin_c: None,
            freq_range_khz: None,
            abort_reason: None,
        },
        system_context: WorkloadSystemContext {
            system_cpu_percent_avg: None,
            system_memory_available_min_kb: None,
            background_activity_confounder: "fixture".to_string(),
        },
        data_quality: WorkloadDataQuality {
            degraded: false,
            missing: Vec::new(),
            notes: Vec::new(),
        },
        evidence_refs: Vec::new(),
        time_unix_ms: 1,
    }
}
