use adc_lab_core::{
    all_claims, artifact_path, claim, claim_definition, Artifact, Decision, EvidenceStore, Kind,
    Status,
};
use schemars::schema_for;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ProbePayload {
    sample_count: u64,
}

#[test]
fn artifact_envelope_rejects_unknown_fields() {
    let artifact = Artifact::new(
        Kind::Pressure,
        "ARTIFACT-001",
        "LAB-RUN-001",
        "target55",
        Status::Measured,
        ProbePayload { sample_count: 1 },
        1,
    );
    let mut value = serde_json::to_value(artifact).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("unknown".to_string(), serde_json::json!(true));
    let error = serde_json::from_value::<Artifact<ProbePayload>>(value).unwrap_err();
    assert!(error.to_string().contains("unknown field"));
}

#[test]
fn generated_artifact_schema_is_strict_at_envelope_level() {
    let schema = serde_json::to_value(schema_for!(Artifact<ProbePayload>)).unwrap();
    assert_eq!(
        schema.get("additionalProperties"),
        Some(&serde_json::json!(false))
    );
}

#[test]
fn claim_catalog_is_single_source_for_blocked_text() {
    let definition = claim_definition(claim::PRODUCTION_READY).unwrap();
    assert!(definition.blocked_text.contains("production readiness"));
    assert!(definition
        .default_next_evidence
        .iter()
        .any(|item| item.contains("operating envelope")));
    assert!(all_claims().len() >= 3);
}

#[test]
fn evidence_store_writes_indexes_and_loads_v2_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let mut artifact = Artifact::new(
        Kind::Pressure,
        "ARTIFACT-001",
        "LAB-RUN-001",
        "target55",
        Status::Measured,
        ProbePayload { sample_count: 3 },
        1,
    );
    artifact.claims.push(adc_lab_core::Claim {
        claim_id: claim::COUPLING_MEMORY_TO_STORAGE.to_string(),
        decision: Decision::Provisional,
        evidence_refs: Vec::new(),
        next_evidence: Vec::new(),
    });

    let artifact_ref = store
        .write(temp.path(), Path::new("pressure/memory.json"), &artifact)
        .unwrap();
    assert_eq!(
        artifact_ref,
        "artifact://lab/runs/LAB-RUN-001/pressure/memory.json"
    );
    assert!(temp.path().join("audit.jsonl").exists());

    let reopened = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap();
    let meta = reopened.iter(Kind::Pressure).next().unwrap();
    let loaded: Artifact<ProbePayload> = reopened.load(meta).unwrap();
    assert_eq!(loaded.payload.sample_count, 3);
}

#[test]
fn evidence_store_rejects_symlink_paths() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target.json");
    fs::write(&target, "{}").unwrap();
    let link = temp.path().join("link.json");
    make_symlink(&target, &link);
    let error = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap_err();
    assert!(error.to_string().contains("symlink"));
}

#[test]
fn evidence_store_reports_malformed_json() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("bad.json"), "{ not json").unwrap();
    let error = EvidenceStore::open(&[temp.path().to_path_buf()]).unwrap_err();
    assert!(error.to_string().contains("malformed JSON artifact"));
}

#[test]
fn runfs_rejects_escape_paths() {
    let temp = tempfile::tempdir().unwrap();
    assert!(artifact_path(temp.path(), "reports/out.json").is_ok());
    assert!(artifact_path(temp.path(), "../outside.json").is_err());
    assert!(artifact_path(temp.path(), "/tmp/outside.json").is_err());
}

#[cfg(unix)]
fn make_symlink(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[cfg(windows)]
fn make_symlink(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).unwrap();
}
