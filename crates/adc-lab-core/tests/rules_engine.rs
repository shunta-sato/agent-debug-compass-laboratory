use adc_lab_core::{
    blocked_claims_for, claim, evaluate_operating_contract_v2, evaluate_rules,
    evaluate_suitability_v2, generate_design_constraint_pack, operating_contract_from_rules_v2,
    Artifact, CompositePayload, Decision, EvidenceStore, Kind, Pred, PressurePayload, Rule, Status,
    SuitabilityDecision, SuitabilityDecisionValue, WorkloadDataQuality,
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
    let decision = SuitabilityDecision {
        schema_version: "lab.suitability_decision.v1".to_string(),
        decision_id: "SUITABILITY-001".to_string(),
        target_id: "target55".to_string(),
        workload_id: "workload-001".to_string(),
        policy_id: "policy-001".to_string(),
        overall_decision: SuitabilityDecisionValue::Unknown,
        selection_ready: false,
        dimensions: Vec::new(),
        blocked_claims: blocked_claims_for(&[
            claim::PRODUCTION_READY,
            claim::REAL_TIME_PRESSURE_SAFE,
        ]),
        data_quality: WorkloadDataQuality {
            degraded: false,
            missing: Vec::new(),
            notes: Vec::new(),
        },
        evidence_refs: Vec::new(),
        time_unix_ms: 1,
    };

    let pack = generate_design_constraint_pack(&decision);

    assert!(pack
        .blocked_claims
        .contains(&"production readiness".to_string()));
    assert!(pack
        .blocked_claims
        .contains(&"real-time safe under all pressure".to_string()));
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
        },
        1,
    );
    store.write(run_dir, Path::new(path), &artifact).unwrap();
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
