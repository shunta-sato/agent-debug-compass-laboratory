use adc_lab_core::*;
use serde_json::json;
use std::fs;
use std::path::Path;

#[test]
fn suitability_policy_unknown_cannot_become_meet() {
    let temp = tempfile::tempdir().unwrap();
    let policy = SuitabilityPolicy {
        schema_version: "lab.suitability_policy.v1".to_string(),
        policy_id: "test".to_string(),
        required_dimensions: vec![SuitabilityDimensionKind::Thermal],
        optional_dimensions: Vec::new(),
        rules: SuitabilityPolicyRules {
            unknown_required_dimension_blocks_selection: true,
            unknown_never_becomes_meet: true,
        },
        thermal: Some(ThermalSuitabilityPolicy {
            max_temp_c: 75.0,
            marginal_margin_c_below: 5.0,
        }),
        cpu: None,
        memory: None,
    };
    let workload = WorkloadDemandProfile {
        schema_version: "lab.workload_demand_profile.v1".to_string(),
        profile_id: "profile".to_string(),
        run_id: "run".to_string(),
        workload_id: "workload".to_string(),
        target_id: "target55".to_string(),
        execution_mode: WorkloadExecutionMode::TargetLocal,
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
            duty_cycle: "bounded_burst".to_string(),
            child_process_accounting_status: "unsupported".to_string(),
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
            background_activity_confounder: "measured_partial".to_string(),
        },
        data_quality: WorkloadDataQuality {
            degraded: false,
            missing: Vec::new(),
            notes: Vec::new(),
        },
        evidence_refs: Vec::new(),
        time_unix_ms: 1,
    };
    let contract = Artifact::new(
        Kind::ReportOperatingContract,
        "OPERATING-CONTRACT-001",
        "run",
        "target55",
        Status::Insufficient,
        OperatingContractPayload {
            rule_set_id: "test.operating_contract".to_string(),
            evaluations: Vec::new(),
            blocked_claims: vec![claim::THERMAL_SUSTAINED_SOAK.to_string()],
            next_evidence: Vec::new(),
        },
        1,
    );
    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &contract,
        &workload,
        &policy,
        SuitabilityArtifactContext {
            target_contract_ref: "contract".to_string(),
            workload_ref: "workload".to_string(),
            policy_ref: "policy".to_string(),
            run_id: "run".to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        decision.payload.overall_decision,
        Some(SuitabilityDecisionValue::Unknown)
    );
    assert!(!decision.payload.selection_ready);
    assert!(decision
        .payload
        .blocked_claims
        .contains(&claim::SELECTION_READY.to_string()));
    assert!(decision
        .payload
        .blocked_claims
        .contains(&claim::THERMAL_SUSTAINED_SOAK.to_string()));
}

#[test]
fn suitability_cpu_memory_require_process_metrics_before_meet() {
    let temp = tempfile::tempdir().unwrap();
    let policy = cpu_memory_policy();
    let workload = workload_profile(false, None, None, Some(8 * 1024 * 1024));
    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    assert_eq!(
        decision.payload.overall_decision,
        Some(SuitabilityDecisionValue::Unknown)
    );
    assert!(!decision.payload.selection_ready);
    assert_dimension_unknown(&decision, SuitabilityDimensionKind::Cpu);
    assert_dimension_unknown(&decision, SuitabilityDimensionKind::Memory);
    assert!(decision
        .payload
        .blocked_claims
        .contains(&claim::SELECTION_READY.to_string()));
}

#[test]
fn suitability_degraded_workload_demand_cannot_support_selection_ready() {
    let temp = tempfile::tempdir().unwrap();
    let policy = cpu_memory_policy();
    let workload = workload_profile(true, Some(5.0), Some(64 * 1024), Some(8 * 1024 * 1024));
    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    assert_eq!(
        decision.payload.overall_decision,
        Some(SuitabilityDecisionValue::Unknown)
    );
    assert!(!decision.payload.selection_ready);
    assert_dimension_unknown(&decision, SuitabilityDimensionKind::Cpu);
    assert_dimension_unknown(&decision, SuitabilityDimensionKind::Memory);
    assert_eq!(decision.data_quality.level, DataQualityLevel::Degraded);
}

#[test]
fn suitability_links_storage_network_latency_pressure_refs() {
    let temp = tempfile::tempdir().unwrap();
    write_pressure_artifact(
        temp.path(),
        "pressure/storage_io.v2.json",
        "storage_io",
        "measured_partial",
        "smoke",
        true,
        None,
        None,
        None,
    );
    write_pressure_artifact(
        temp.path(),
        "pressure/network_io.endpoint.v2.json",
        "network_io",
        "measured_partial",
        "boundary_probe",
        true,
        Some("bounded_transfer"),
        Some(true),
        Some(4096),
    );
    write_pressure_artifact(
        temp.path(),
        "pressure/latency_jitter.v2.json",
        "latency_jitter",
        "measured_partial",
        "smoke",
        false,
        None,
        None,
        None,
    );
    let policy = resource_policy(vec![
        SuitabilityDimensionKind::StorageIo,
        SuitabilityDimensionKind::NetworkIo,
        SuitabilityDimensionKind::LatencyJitter,
    ]);
    let workload = workload_profile(false, None, None, None);

    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    assert_eq!(
        decision.payload.overall_decision,
        Some(SuitabilityDecisionValue::Marginal)
    );
    assert!(decision.payload.selection_ready);
    assert_dimension_marginal_with_ref(
        &decision,
        SuitabilityDimensionKind::StorageIo,
        "target-run://pressure/storage_io.v2.json",
    );
    assert_dimension_marginal_with_ref(
        &decision,
        SuitabilityDimensionKind::NetworkIo,
        "target-run://pressure/network_io.endpoint.v2.json",
    );
    assert_dimension_marginal_with_ref(
        &decision,
        SuitabilityDimensionKind::LatencyJitter,
        "target-run://pressure/latency_jitter.v2.json",
    );
}

#[test]
fn suitability_counter_only_network_blocks_required_selection() {
    let temp = tempfile::tempdir().unwrap();
    write_pressure_artifact(
        temp.path(),
        "pressure/network_io.counter_only.v2.json",
        "network_io",
        "not_applicable",
        "not_applicable",
        false,
        Some("counter_only"),
        Some(false),
        Some(0),
    );
    let policy = resource_policy(vec![SuitabilityDimensionKind::NetworkIo]);
    let workload = workload_profile(false, None, None, None);

    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    assert!(!decision.payload.selection_ready);
    let network = dimension_decision(&decision, SuitabilityDimensionKind::NetworkIo);
    assert_eq!(network.decision, SuitabilityDecisionValue::Unknown);
    assert_ne!(network.decision, SuitabilityDecisionValue::Meet);
    assert!(network
        .evidence_refs
        .contains(&"target-run://pressure/network_io.counter_only.v2.json".to_string()));
    assert!(network
        .unknown_reason
        .as_deref()
        .unwrap_or_default()
        .contains("counter-only"));
}

#[test]
fn suitability_storage_smoke_cannot_become_device_meet() {
    let temp = tempfile::tempdir().unwrap();
    write_pressure_artifact(
        temp.path(),
        "pressure/storage_io.v2.json",
        "storage_io",
        "measured_partial",
        "smoke",
        true,
        None,
        None,
        None,
    );
    let policy = resource_policy(vec![SuitabilityDimensionKind::StorageIo]);
    let workload = workload_profile(false, None, None, None);

    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    let storage = dimension_decision(&decision, SuitabilityDimensionKind::StorageIo);
    assert_eq!(storage.decision, SuitabilityDecisionValue::Marginal);
    assert_ne!(storage.decision, SuitabilityDecisionValue::Meet);
    assert!(storage
        .margin
        .as_deref()
        .unwrap_or_default()
        .contains("not a storage-device meet"));
}

#[test]
fn suitability_required_missing_latency_still_blocks_selection() {
    let temp = tempfile::tempdir().unwrap();
    let policy = resource_policy(vec![SuitabilityDimensionKind::LatencyJitter]);
    let workload = workload_profile(false, None, None, None);

    let decision = decide_suitability_artifact_v2(
        temp.path(),
        &empty_contract(),
        &workload,
        &policy,
        test_context(),
    )
    .unwrap();

    assert!(!decision.payload.selection_ready);
    assert_dimension_unknown(&decision, SuitabilityDimensionKind::LatencyJitter);
}

fn cpu_memory_policy() -> SuitabilityPolicy {
    SuitabilityPolicy {
        schema_version: "lab.suitability_policy.v1".to_string(),
        policy_id: "test".to_string(),
        required_dimensions: vec![
            SuitabilityDimensionKind::Cpu,
            SuitabilityDimensionKind::Memory,
        ],
        optional_dimensions: Vec::new(),
        rules: SuitabilityPolicyRules {
            unknown_required_dimension_blocks_selection: true,
            unknown_never_becomes_meet: true,
        },
        thermal: None,
        cpu: Some(CpuSuitabilityPolicy {
            max_process_cpu_percent_avg: 80.0,
        }),
        memory: Some(MemorySuitabilityPolicy {
            min_memory_margin_mb: 1024,
        }),
    }
}

fn resource_policy(required_dimensions: Vec<SuitabilityDimensionKind>) -> SuitabilityPolicy {
    SuitabilityPolicy {
        schema_version: "lab.suitability_policy.v1".to_string(),
        policy_id: "resource-test".to_string(),
        required_dimensions,
        optional_dimensions: Vec::new(),
        rules: SuitabilityPolicyRules {
            unknown_required_dimension_blocks_selection: true,
            unknown_never_becomes_meet: true,
        },
        thermal: None,
        cpu: None,
        memory: None,
    }
}

fn workload_profile(
    degraded: bool,
    process_cpu_percent_avg: Option<f64>,
    rss_peak_kb: Option<u64>,
    system_memory_available_min_kb: Option<u64>,
) -> WorkloadDemandProfile {
    WorkloadDemandProfile {
        schema_version: "lab.workload_demand_profile.v1".to_string(),
        profile_id: "profile".to_string(),
        run_id: "run".to_string(),
        workload_id: "workload".to_string(),
        target_id: "target55".to_string(),
        execution_mode: WorkloadExecutionMode::TargetLocal,
        demand_scope: WorkloadDemandScope::ProcessScoped,
        workload_demand: WorkloadDemand {
            process_cpu_utime_ticks: None,
            process_cpu_stime_ticks: None,
            process_cpu_time_ms: process_cpu_percent_avg,
            process_cpu_percent_avg,
            process_cpu_percent_peak: process_cpu_percent_avg,
            rss_peak_kb,
            vmhwm_peak_kb: rss_peak_kb,
            read_bytes: None,
            write_bytes: None,
            cancelled_write_bytes: None,
            voluntary_ctxt_switches: None,
            nonvoluntary_ctxt_switches: None,
            duty_cycle: "bounded_burst".to_string(),
            child_process_accounting_status: "unsupported".to_string(),
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
            system_memory_available_min_kb,
            background_activity_confounder: "measured_partial".to_string(),
        },
        data_quality: WorkloadDataQuality {
            degraded,
            missing: if degraded {
                vec!["workload run incomplete".to_string()]
            } else {
                Vec::new()
            },
            notes: Vec::new(),
        },
        evidence_refs: Vec::new(),
        time_unix_ms: 1,
    }
}

#[allow(clippy::too_many_arguments)]
fn write_pressure_artifact(
    run_dir: &Path,
    relative_path: &str,
    pressure_kind: &str,
    status_state: &str,
    evidence_class: &str,
    effect_observed: bool,
    network_mode: Option<&str>,
    network_endpoint_available: Option<bool>,
    network_traffic_generated_bytes: Option<u64>,
) {
    let path = run_dir.join(relative_path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let artifact = json!({
        "schema": "lab.artifact.v2",
        "kind": "pressure",
        "id": format!("PRESSURE-{pressure_kind}"),
        "run_id": "run",
        "target_id": "target55",
        "status": {
            "state": status_state
        },
        "bounds": null,
        "factors": {
            "controlled": [],
            "observed": [],
            "confounders": []
        },
        "metrics": [],
        "claims": [],
        "evidence_refs": [],
        "data_quality": {
            "level": "partial",
            "notes": []
        },
        "payload": {
            "source_schema_version": "lab.resource_pressure_result.v1",
            "pressure_kind": pressure_kind,
            "evidence_class": evidence_class,
            "effect_observed": effect_observed,
            "duration_ms": 1000,
            "network_mode": network_mode,
            "network_endpoint_available": network_endpoint_available,
            "network_traffic_generated_bytes": network_traffic_generated_bytes
        },
        "time_unix_ms": 1
    });
    fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
}

fn empty_contract() -> Artifact<OperatingContractPayload> {
    Artifact::new(
        Kind::ReportOperatingContract,
        "OPERATING-CONTRACT-001",
        "run",
        "target55",
        Status::Insufficient,
        OperatingContractPayload {
            rule_set_id: "test.operating_contract".to_string(),
            evaluations: Vec::new(),
            blocked_claims: Vec::new(),
            next_evidence: Vec::new(),
        },
        1,
    )
}

fn test_context() -> SuitabilityArtifactContext {
    SuitabilityArtifactContext {
        target_contract_ref: "contract".to_string(),
        workload_ref: "workload".to_string(),
        policy_ref: "policy".to_string(),
        run_id: "run".to_string(),
    }
}

fn dimension_decision(
    decision: &Artifact<SuitabilityPayload>,
    dimension: SuitabilityDimensionKind,
) -> &SuitabilityDimensionDecision {
    decision
        .payload
        .dimensions
        .iter()
        .find(|candidate| candidate.dimension == dimension)
        .unwrap()
}

fn assert_dimension_unknown(
    decision: &Artifact<SuitabilityPayload>,
    dimension: SuitabilityDimensionKind,
) {
    let dimension = dimension_decision(decision, dimension);
    assert_eq!(dimension.decision, SuitabilityDecisionValue::Unknown);
    assert!(dimension.unknown_reason.is_some());
}

fn assert_dimension_marginal_with_ref(
    decision: &Artifact<SuitabilityPayload>,
    dimension: SuitabilityDimensionKind,
    evidence_ref: &str,
) {
    let dimension = dimension_decision(decision, dimension);
    assert_eq!(dimension.decision, SuitabilityDecisionValue::Marginal);
    assert!(dimension.evidence_refs.contains(&evidence_ref.to_string()));
}
