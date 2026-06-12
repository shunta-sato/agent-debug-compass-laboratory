use adc_lab_core::{
    claim, evaluate_operating_contract_v2, evaluate_rules, evaluate_suitability_v2,
    generate_constraints_artifact_v2, operating_contract_from_rules_v2,
    render_agent_constraints_markdown, Artifact, CompositePayload, Decision, EvidenceStore,
    GovernorValidation, GovernorValidity, Kind, Pred, PressurePayload, Rule, RunValidationPayload,
    Status, SuitabilityDecisionValue, SuitabilityPayload, FULLSET_PROFILE,
};
use std::path::Path;

#[test]
fn rules_engine_table_row_changes_core_output() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_marker(
        &mut store,
        temp.path(),
        Kind::Pressure,
        "pressure/memory.json",
    );

    let baseline = evaluate_rules(
        &store,
        &[Rule {
            id: "test.pressure_present",
            claim_id: claim::COUPLING_MEMORY_TO_STORAGE,
            when: Pred::Present(Kind::Pressure),
            on_match: Decision::Supported,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::Pressure],
            next_evidence: &["collect pressure evidence"],
        }],
    );
    let with_one_more_row = evaluate_rules(
        &store,
        &[
            Rule {
                id: "test.pressure_present",
                claim_id: claim::COUPLING_MEMORY_TO_STORAGE,
                when: Pred::Present(Kind::Pressure),
                on_match: Decision::Supported,
                on_miss: Decision::Blocked,
                evidence_kinds: &[Kind::Pressure],
                next_evidence: &["collect pressure evidence"],
            },
            Rule {
                id: "test.composite_present",
                claim_id: claim::PRODUCTION_READY,
                when: Pred::Present(Kind::Composite),
                on_match: Decision::Provisional,
                on_miss: Decision::Blocked,
                evidence_kinds: &[Kind::Composite],
                next_evidence: &["collect composite evidence"],
            },
        ],
    );

    assert_eq!(baseline.len(), 1);
    assert_eq!(with_one_more_row.len(), 2);
    assert_eq!(with_one_more_row[1].decision, Decision::Blocked);
    assert_eq!(with_one_more_row[1].missing, vec!["composite"]);
}

#[test]
fn operating_contract_v2_uses_rule_set_and_stays_core_only() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_pressure(
        &mut store,
        temp.path(),
        "pressure/memory.json",
        Status::MeasuredPartial,
        true,
    );
    write_composite(
        &mut store,
        temp.path(),
        "composite/memory_storage.json",
        Status::MeasuredPartial,
        "composite_measured",
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");

    assert_eq!(contract.schema, "lab.artifact.v2");
    assert_eq!(contract.kind, Kind::ReportOperatingContract);
    assert_eq!(
        contract
            .claims
            .iter()
            .find(|entry| entry.claim_id == claim::COUPLING_MEMORY_TO_STORAGE)
            .unwrap()
            .decision,
        Decision::Supported
    );
    assert!(contract
        .payload
        .blocked_claims
        .contains(&claim::PRODUCTION_READY.to_string()));
}

#[test]
fn operating_contract_coupling_requires_measured_effect_not_just_presence() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_pressure(
        &mut store,
        temp.path(),
        "pressure/memory.json",
        Status::MeasuredPartial,
        true,
    );
    write_composite(
        &mut store,
        temp.path(),
        "composite/memory_storage.json",
        Status::Insufficient,
        "composite_measured",
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let coupling = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.memory_storage_coupling_requires_composite")
        .unwrap();

    assert!(!coupling.matched);
    assert_eq!(coupling.decision, Decision::Blocked);
    assert!(contract
        .payload
        .blocked_claims
        .contains(&claim::COUPLING_MEMORY_TO_STORAGE.to_string()));
}

#[test]
fn operating_contract_covers_core_boundary_claims_conservatively() {
    let temp = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let rule_ids = contract
        .payload
        .evaluations
        .iter()
        .map(|entry| entry.rule_id.as_str())
        .collect::<Vec<_>>();

    for expected in [
        "operating.memory_storage_coupling_requires_composite",
        "operating.sustained_thermal_requires_soak",
        "operating.storage_default_writes_require_bounded_probe",
        "operating.network_background_io_requires_bounded_transfer",
        "operating.real_time_pressure_requires_jitter_evidence",
        "operating.observer_cadence_requires_bounded_samples",
        "operating.production_readiness_requires_run_report",
    ] {
        assert!(rule_ids.contains(&expected), "missing rule {expected}");
    }
    for claim_id in [
        claim::THERMAL_SUSTAINED_SOAK,
        claim::NETWORK_BOUNDED_TRANSFER,
        claim::OBSERVER_CADENCE_BOUNDED,
        claim::REAL_TIME_PRESSURE_SAFE,
    ] {
        assert!(
            contract
                .payload
                .blocked_claims
                .contains(&claim_id.to_string()),
            "missing blocked claim {claim_id}"
        );
    }
}

#[test]
fn operating_contract_network_boundary_requires_bounded_transfer_payload() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_network_pressure(&mut store, temp.path());

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let network = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.network_background_io_requires_bounded_transfer")
        .unwrap();

    assert!(network.matched);
    assert_eq!(network.decision, Decision::Provisional);
    assert!(!contract
        .payload
        .blocked_claims
        .contains(&claim::NETWORK_BOUNDED_TRANSFER.to_string()));
}

#[test]
fn operating_contract_requires_measured_run_validation_for_production_readiness() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_marker(
        &mut store,
        temp.path(),
        Kind::ReportRun,
        "reports/run_report.v2.json",
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let production_ready = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.production_readiness_requires_run_report")
        .unwrap();

    assert!(!production_ready.matched);
    assert_eq!(production_ready.decision, Decision::Blocked);
    assert!(contract
        .payload
        .blocked_claims
        .contains(&claim::PRODUCTION_READY.to_string()));
}

#[test]
fn operating_contract_blocks_contaminated_run_validation_for_production_readiness() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_marker(
        &mut store,
        temp.path(),
        Kind::ReportRun,
        "reports/run_report.v2.json",
    );
    write_run_validation(
        &mut store,
        temp.path(),
        "reports/run_validation.v2.json",
        GovernorValidity::Contaminated,
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let production_ready = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.production_readiness_requires_run_report")
        .unwrap();

    assert!(!production_ready.matched);
    assert_eq!(production_ready.decision, Decision::Blocked);
    assert!(contract
        .payload
        .blocked_claims
        .contains(&claim::PRODUCTION_READY.to_string()));
}

#[test]
fn operating_contract_allows_production_readiness_only_with_measured_run_validation() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_marker(
        &mut store,
        temp.path(),
        Kind::ReportRun,
        "reports/run_report.v2.json",
    );
    write_run_validation(
        &mut store,
        temp.path(),
        "reports/run_validation.v2.json",
        GovernorValidity::Measured,
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let production_ready = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.production_readiness_requires_run_report")
        .unwrap();

    assert!(production_ready.matched);
    assert_eq!(production_ready.decision, Decision::Provisional);
    assert!(!contract
        .payload
        .blocked_claims
        .contains(&claim::PRODUCTION_READY.to_string()));
}

#[test]
fn operating_contract_blocks_mixed_measured_and_contaminated_run_validation() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    write_marker(
        &mut store,
        temp.path(),
        Kind::ReportRun,
        "reports/run_report.v2.json",
    );
    write_run_validation(
        &mut store,
        temp.path(),
        "reports/run_validation-measured.v2.json",
        GovernorValidity::Measured,
    );
    write_run_validation(
        &mut store,
        temp.path(),
        "reports/run_validation-contaminated.v2.json",
        GovernorValidity::Contaminated,
    );

    let contract = evaluate_operating_contract_v2(&store, "LAB-RUN-001", "target55");
    let production_ready = contract
        .payload
        .evaluations
        .iter()
        .find(|entry| entry.rule_id == "operating.production_readiness_requires_run_report")
        .unwrap();

    assert!(!production_ready.matched);
    assert_eq!(production_ready.decision, Decision::Blocked);
}

#[test]
fn operating_contract_custom_rule_row_changes_payload_without_generator() {
    let temp = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let artifact = operating_contract_from_rules_v2(
        &store,
        "rules.operating_contract.test",
        vec![Rule {
            id: "test.report_run_required",
            claim_id: claim::PRODUCTION_READY,
            when: Pred::Present(Kind::ReportRun),
            on_match: Decision::Provisional,
            on_miss: Decision::Blocked,
            evidence_kinds: &[Kind::ReportRun],
            next_evidence: &["generate v2 run report"],
        }],
        "LAB-RUN-001",
        "target55",
    );

    assert_eq!(artifact.payload.evaluations.len(), 1);
    assert_eq!(
        artifact.payload.evaluations[0].rule_id,
        "test.report_run_required"
    );
    assert_eq!(artifact.payload.evaluations[0].decision, Decision::Blocked);
}

#[test]
fn suitability_v2_is_conservative_without_required_v2_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();

    let suitability = evaluate_suitability_v2(&store, "LAB-RUN-001", "target55");

    assert_eq!(suitability.kind, Kind::ReportSuitability);
    assert_eq!(suitability.status, Status::Insufficient);
    assert!(!suitability.payload.selection_ready);
    assert!(suitability
        .payload
        .blocked_claims
        .contains(&claim::TARGET_SELECTION_PI4_SUFFICIENT.to_string()));
}

#[test]
fn constraint_pack_blocked_claims_come_from_catalog_terms() {
    let suitability = Artifact::new(
        Kind::ReportSuitability,
        "SUITABILITY-001",
        "LAB-RUN-001",
        "target55",
        Status::Insufficient,
        SuitabilityPayload {
            rule_set_id: "test.suitability".to_string(),
            selection_ready: false,
            workload_id: Some("workload-001".to_string()),
            policy_id: Some("policy-001".to_string()),
            overall_decision: Some(SuitabilityDecisionValue::Unknown),
            dimensions: Vec::new(),
            evaluations: Vec::new(),
            blocked_claims: vec![
                claim::PRODUCTION_READY.to_string(),
                claim::REAL_TIME_PRESSURE_SAFE.to_string(),
            ],
            next_evidence: Vec::new(),
        },
        1,
    );

    let constraints = generate_constraints_artifact_v2(&suitability);
    let markdown = render_agent_constraints_markdown(&constraints, "artifact://test");

    assert!(constraints
        .payload
        .blocked_claims
        .contains(&claim::PRODUCTION_READY.to_string()));
    assert!(constraints
        .payload
        .blocked_claims
        .contains(&claim::REAL_TIME_PRESSURE_SAFE.to_string()));
    assert!(markdown.contains("production readiness"));
    assert!(markdown.contains("real-time safe under all pressure"));
}

fn write_marker(store: &mut EvidenceStore, run_dir: &Path, kind: Kind, path: &str) {
    let artifact = Artifact::new(
        kind,
        format!("ARTIFACT-{kind:?}"),
        "LAB-RUN-001",
        "target55",
        Status::Measured,
        serde_json::json!({ "marker": format!("{kind:?}") }),
        1,
    );
    store.write(run_dir, Path::new(path), &artifact).unwrap();
}

fn write_run_validation(
    store: &mut EvidenceStore,
    run_dir: &Path,
    path: &str,
    validity: GovernorValidity,
) {
    let status = match validity {
        GovernorValidity::Measured => Status::Measured,
        GovernorValidity::MeasuredPartial => Status::MeasuredPartial,
        GovernorValidity::Refused => Status::Refused {
            code: adc_lab_core::EvidenceRefusalCode::PolicyViolation,
            message: "refused control evidence".to_string(),
        },
        GovernorValidity::Contaminated => Status::UnsafeBlocked {
            reason: "contaminated control evidence".to_string(),
        },
        GovernorValidity::NotApplicable => Status::NotApplicable {
            reason: "not applicable".to_string(),
        },
        GovernorValidity::Insufficient | GovernorValidity::Unknown => Status::Insufficient,
    };
    let artifact = Artifact::new(
        Kind::ReportRunValidation,
        format!("RUN-VALIDATION-{path}"),
        "LAB-RUN-001",
        "target55",
        status,
        RunValidationPayload {
            profile: FULLSET_PROFILE.to_string(),
            requested_governors: vec!["performance".to_string()],
            governor_results: vec![GovernorValidation {
                governor: "performance".to_string(),
                validity: validity.clone(),
                plan_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/plans/performance.json".to_string(),
                ),
                approval_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/approvals/performance.json".to_string(),
                ),
                control_result_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/plans/performance.result.json".to_string(),
                ),
                load_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/load/performance.v2.json".to_string(),
                ),
                restore_result_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/restore/performance.result.json".to_string(),
                ),
                health_check_ref: Some(
                    "artifact://lab/runs/LAB-RUN-001/health/restore_health_check.json".to_string(),
                ),
                messages: vec!["test validation".to_string()],
                next_evidence: Vec::new(),
            }],
            overall_validity: validity,
            gaps: Vec::new(),
            audit_refs: vec!["artifact://lab/runs/LAB-RUN-001/audit.jsonl".to_string()],
        },
        1,
    );
    store.write(run_dir, Path::new(path), &artifact).unwrap();
}

fn write_pressure(
    store: &mut EvidenceStore,
    run_dir: &Path,
    path: &str,
    status: Status,
    effect_observed: bool,
) {
    let artifact = Artifact::new(
        Kind::Pressure,
        format!("ARTIFACT-{path}"),
        "LAB-RUN-001",
        "target55",
        status,
        PressurePayload {
            source_schema_version: "lab.resource_pressure_result.v1".to_string(),
            pressure_kind: "memory_pressure".to_string(),
            evidence_class: "pressure_induced".to_string(),
            effect_observed,
            duration_ms: 100,
            network_mode: None,
            network_endpoint_available: None,
            network_traffic_generated_bytes: None,
        },
        1,
    );
    store.write(run_dir, Path::new(path), &artifact).unwrap();
}

fn write_network_pressure(store: &mut EvidenceStore, run_dir: &Path) {
    let artifact = Artifact::new(
        Kind::Pressure,
        "ARTIFACT-network",
        "LAB-RUN-001",
        "target55",
        Status::MeasuredPartial,
        PressurePayload {
            source_schema_version: "lab.resource_pressure_result.v1".to_string(),
            pressure_kind: "network_io".to_string(),
            evidence_class: "boundary_probe".to_string(),
            effect_observed: true,
            duration_ms: 100,
            network_mode: Some("bounded_transfer".to_string()),
            network_endpoint_available: Some(true),
            network_traffic_generated_bytes: Some(4096),
        },
        1,
    );
    store
        .write(run_dir, Path::new("pressure/network.json"), &artifact)
        .unwrap();
}

fn write_composite(
    store: &mut EvidenceStore,
    run_dir: &Path,
    path: &str,
    status: Status,
    coupling_evidence_class: &str,
) {
    let artifact = Artifact::new(
        Kind::Composite,
        format!("ARTIFACT-{path}"),
        "LAB-RUN-001",
        "target55",
        status,
        CompositePayload {
            source_schema_version: "lab.composite_boundary_result.v1".to_string(),
            scenario: "memory_storage_jitter".to_string(),
            coupling_evidence_class: coupling_evidence_class.to_string(),
            phase_count: 3,
            duration_ms: 100,
        },
        1,
    );
    store.write(run_dir, Path::new(path), &artifact).unwrap();
}
