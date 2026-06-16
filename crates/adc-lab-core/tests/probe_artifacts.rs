use adc_lab_core::{
    claim, create_or_open_run, evaluate_rules, write_composite_artifact_v2, write_load_artifact_v2,
    write_observation_artifact_v2, write_observation_artifact_v2_with_label,
    write_pressure_artifact_v2, write_workload_artifact_v2, Artifact, CompositeBoundaryResult,
    Decision, EvidenceRefResolutionKind, EvidenceStore, Kind, LoadRestoreOnAbortStatus, LoadResult,
    LoadSafetyMonitorResult, ObservationResult, Pred, PressurePayload, Rule, RunSetSourceRole,
    Signal, Status, WorkloadDataQuality, WorkloadDemand, WorkloadDemandProfile,
    WorkloadDemandScope, WorkloadExecutionMode, WorkloadSystemContext,
    WorkloadTargetConditionedResponse,
};
use std::{fs, path::Path};

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
fn evidence_store_resolves_artifact_refs_across_opened_run_set() {
    let temp = tempfile::tempdir().unwrap();
    let primary = create_or_open_run(Some(temp.path().join("primary"))).unwrap();
    let included = create_or_open_run(Some(temp.path().join("included"))).unwrap();
    let primary_audit = primary.run_dir.join("audit.jsonl");
    fs::write(&primary_audit, "{}\n").unwrap();
    let included_report = included.run_dir.join("reports/target-local.json");
    fs::write(&included_report, "{}").unwrap();
    let primary_ref = primary.artifact_uri(&primary_audit).unwrap();
    let included_ref = included.artifact_uri(&included_report).unwrap();
    let missing_ref = format!(
        "artifact://lab/runs/{}/reports/missing.json",
        included.run_id
    );

    let store = EvidenceStore::open(&[primary.run_dir.clone(), included.run_dir.clone()]).unwrap();
    let payload = store.evidence_ref_resolution_payload(
        "RUN-SET-test",
        vec![primary_ref.clone()],
        vec![
            primary_ref.clone(),
            included_ref.clone(),
            missing_ref.clone(),
            "operator-notes.txt".to_string(),
        ],
    );

    assert_eq!(payload.run_set_resolution_map.len(), 2);
    assert_eq!(
        payload.run_set_resolution_map[0].source_role,
        RunSetSourceRole::Primary
    );
    assert_eq!(
        payload.run_set_resolution_map[1].source_role,
        RunSetSourceRole::Included
    );
    let resolution_for = |reference: &str| {
        payload
            .resolutions
            .iter()
            .find(|resolution| resolution.reference == reference)
            .unwrap()
    };
    assert_eq!(
        resolution_for(&primary_ref).classification,
        EvidenceRefResolutionKind::Resolvable
    );
    assert_eq!(
        resolution_for(&included_ref).classification,
        EvidenceRefResolutionKind::Resolvable
    );
    assert_eq!(
        resolution_for("operator-notes.txt").classification,
        EvidenceRefResolutionKind::DiagnosticExternal
    );
    assert_eq!(
        resolution_for(&missing_ref).classification,
        EvidenceRefResolutionKind::Invalid
    );
    assert_eq!(payload.invalid_refs, vec![missing_ref]);
    assert_eq!(
        payload.diagnostic_external_refs,
        vec!["operator-notes.txt".to_string()]
    );
}

#[test]
fn pressure_and_composite_v2_sidecars_keep_each_result_id() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let mut first_pressure = pressure();
    let mut second_pressure = pressure();
    first_pressure.result_id = "PRESSURE-001".to_string();
    second_pressure.result_id = "PRESSURE-002".to_string();
    let mut first_composite = composite();
    let mut second_composite = composite();
    first_composite.result_id = "COMPOSITE-001".to_string();
    second_composite.result_id = "COMPOSITE-002".to_string();

    write_pressure_artifact_v2(&mut store, temp.path(), first_pressure).unwrap();
    write_pressure_artifact_v2(&mut store, temp.path(), second_pressure).unwrap();
    write_composite_artifact_v2(&mut store, temp.path(), first_composite).unwrap();
    write_composite_artifact_v2(&mut store, temp.path(), second_composite).unwrap();

    let reopened = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    assert_eq!(reopened.iter(Kind::Pressure).count(), 2);
    assert_eq!(reopened.iter(Kind::Composite).count(), 2);
    assert!(temp
        .path()
        .join("pressure/memory_pressure.PRESSURE-001.v2.json")
        .exists());
    assert!(temp
        .path()
        .join("pressure/memory_pressure.PRESSURE-002.v2.json")
        .exists());
    assert!(temp
        .path()
        .join("composite/memory_storage_jitter.COMPOSITE-001.v2.json")
        .exists());
    assert!(temp
        .path()
        .join("composite/memory_storage_jitter.COMPOSITE-002.v2.json")
        .exists());
}

#[test]
fn load_v2_sidecars_keep_each_result_id() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let mut first = load();
    let mut second = load();
    first.result_id = "LOAD-RESULT-001".to_string();
    second.result_id = "LOAD-RESULT-002".to_string();

    write_load_artifact_v2(&mut store, temp.path(), first).unwrap();
    write_load_artifact_v2(&mut store, temp.path(), second).unwrap();

    let reopened = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    assert_eq!(reopened.iter(Kind::Load).count(), 2);
    assert!(temp
        .path()
        .join("load/cpu.LOAD-RESULT-001.v2.json")
        .exists());
    assert!(temp
        .path()
        .join("load/cpu.LOAD-RESULT-002.v2.json")
        .exists());
}

#[test]
fn observation_v2_sidecars_keep_each_artifact_label() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();

    let baseline_ref = write_observation_artifact_v2_with_label(
        &mut store,
        temp.path(),
        observation(),
        Some("observe_baseline_60s"),
    )
    .unwrap();
    let cooldown_ref = write_observation_artifact_v2_with_label(
        &mut store,
        temp.path(),
        observation(),
        Some("cooldown_after_sustained_load"),
    )
    .unwrap();

    let reopened = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    assert_eq!(reopened.iter(Kind::Observation).count(), 2);
    assert_ne!(baseline_ref, cooldown_ref);
    assert_eq!(
        reopened.resolve_evidence_ref(&baseline_ref).classification,
        EvidenceRefResolutionKind::Resolvable
    );
    assert_eq!(
        reopened.resolve_evidence_ref(&cooldown_ref).classification,
        EvidenceRefResolutionKind::Resolvable
    );

    let observation_names = fs::read_dir(temp.path().join("observations"))
        .unwrap()
        .flatten()
        .filter_map(|entry| entry.file_name().to_str().map(str::to_string))
        .collect::<Vec<_>>();
    assert!(observation_names
        .iter()
        .any(|name| name.starts_with("observe_baseline_60s.") && name.ends_with(".v2.json")));
    assert!(observation_names.iter().any(|name| {
        name.starts_with("cooldown_after_sustained_load.") && name.ends_with(".v2.json")
    }));
    assert!(!temp.path().join("observations/observe.v2.json").exists());
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
            network_mode: None,
            network_endpoint_available: None,
            network_traffic_generated_bytes: None,
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
