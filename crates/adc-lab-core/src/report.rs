use crate::contracts::{
    BuildInfo, ExperimentRun, ReleaseManifest, RunArtifactRef, RunDataQuality, RunManifest,
};
use crate::{artifact_uri_for_run, run_id_from_run_dir, LabResult};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

const READ_ONLY_REQUIRED_AUDIT_OPS: &[&str] = &[
    "inventory",
    "toolchain.discover",
    "tool.qualify_inventory",
    "observe",
];

pub(crate) const OP_INVENTORY: &str = "inventory";
pub(crate) const OP_TOOLCHAIN_DISCOVERY: &str = "toolchain_discovery";
pub(crate) const OP_PASSIVE_OBSERVE: &str = "passive_observe";
pub(crate) const OP_BOUNDED_LOAD: &str = "bounded_load";
pub(crate) const OP_PRIVILEGED_CONTROL: &str = "privileged_control";
pub(crate) const OP_CONTROLLED_OPERATING_POINT: &str = "controlled_operating_point";
pub(crate) const OP_SUSTAINED_THERMAL: &str = "sustained_thermal";
pub(crate) const STATUS_COMPLETED: &str = "completed";
pub(crate) const STATUS_NOT_RUN: &str = "not_run";

#[derive(Debug, Clone)]
pub(crate) struct RunEvidenceSummary {
    pub(crate) run_id: String,
    pub(crate) target_inventory_ref: Option<String>,
    pub(crate) toolchain_inventory_ref: Option<String>,
    pub(crate) tool_qualification_summary_ref: Option<String>,
    pub(crate) observation_ref: Option<String>,
    pub(crate) load_result_refs: Vec<String>,
    pub(crate) operations_summary: BTreeMap<String, String>,
    pub(crate) operation_audit_refs: BTreeMap<String, String>,
    pub(crate) audit_event_count: usize,
    pub(crate) audit_run_id_mismatches: Vec<String>,
}

pub fn run_manifest(
    run_dir: impl AsRef<Path>,
    target_id: String,
    target: String,
    started_at_unix_ms: u64,
    ended_at_unix_ms: u64,
    controller_build_info: BuildInfo,
) -> LabResult<RunManifest> {
    let run_dir = run_dir.as_ref();
    let summary = run_evidence_summary(run_dir)?;
    let run_id = summary.run_id.clone();
    let artifacts = known_run_artifacts(run_dir, &run_id)?;
    let audit_ref = artifact_ref_if_exists(run_dir, &run_id, "audit.jsonl")?
        .unwrap_or_else(|| format!("artifact://lab/runs/{run_id}/audit.jsonl"));
    let claim_trace_ref = artifact_ref_if_exists(run_dir, &run_id, "reports/run_report.v2.json")?;
    let identity = release_identity(run_dir, &controller_build_info)?;
    let data_quality = run_data_quality(run_dir, &summary, &identity)?;
    Ok(RunManifest {
        schema_version: "lab.run_manifest.v1".to_string(),
        run_id,
        target_id,
        target,
        mode: run_mode(&summary),
        started_at_unix_ms,
        ended_at_unix_ms,
        adc_lab_version: controller_build_info.version,
        adc_lab_git_sha: controller_build_info.git_sha,
        adc_lab_target_version: identity.adc_lab_target_version,
        adc_lab_target_git_sha: identity.adc_lab_target_git_sha,
        release_tag: identity.release_tag,
        release_asset: identity.release_asset,
        release_asset_sha256: identity.release_asset_sha256,
        binary_sha256: identity.binary_sha256,
        operations_summary: summary.operations_summary,
        operation_audit_refs: summary.operation_audit_refs,
        artifacts,
        audit_ref,
        claim_trace_ref,
        data_quality,
    })
}

pub(crate) fn read_json_artifact_if_exists<T: DeserializeOwned>(
    path: PathBuf,
) -> LabResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(&path)?;
    let value = serde_json::from_slice(&bytes).map_err(|error| {
        crate::LabError::Validation(format!(
            "failed to parse JSON artifact {}: {error}",
            path.display()
        ))
    })?;
    Ok(Some(value))
}

fn load_result_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    Ok(collect_artifact_refs(run_dir, run_id)?
        .into_iter()
        .filter(|artifact| {
            (artifact.contains("/load/") && artifact.ends_with(".v2.json"))
                || artifact.ends_with("/load_result.json")
                || (artifact.contains("/loads/") && artifact.ends_with(".result.json"))
        })
        .collect())
}

pub(crate) fn collect_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_files(run_dir, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| artifact_uri_for_run(run_id, run_dir, path))
        .collect()
}

fn run_id_from_dir(run_dir: &Path) -> String {
    run_id_from_run_dir(run_dir)
}

pub(crate) fn artifact_ref_if_exists(
    run_dir: &Path,
    run_id: &str,
    relative_path: &str,
) -> LabResult<Option<String>> {
    let path = run_dir.join(relative_path);
    if path.exists() {
        artifact_uri_for_run(run_id, run_dir, path).map(Some)
    } else {
        Ok(None)
    }
}

fn known_run_artifacts(run_dir: &Path, run_id: &str) -> LabResult<Vec<RunArtifactRef>> {
    let mut artifacts = Vec::new();
    for (name, relative_path, schema_version) in [
        ("run_context", "run_context.json", "lab.run_context.v1"),
        (
            "target_inventory",
            "inventory/target_inventory.json",
            "lab.target_inventory.v1",
        ),
        (
            "toolchain_inventory",
            "toolchain/toolchain_inventory.json",
            "lab.toolchain_inventory.v1",
        ),
        (
            "passive_observe",
            "observations/observe.json",
            "lab.observation_result.v1",
        ),
        (
            "report_run",
            "reports/run_report.v2.json",
            "lab.artifact.v2",
        ),
        (
            "tool_qualification_summary",
            "tools/tool_qualification_summary.json",
            "lab.tool_qualification_summary.v1",
        ),
        (
            "adc_lab_version",
            "tools/adc-lab.version.json",
            "lab.build_info.v1",
        ),
        (
            "adc_lab_target_version",
            "tools/adc-lab-target.version.json",
            "lab.build_info.v1",
        ),
        ("audit_log", "audit.jsonl", "lab.audit_event.v1"),
    ] {
        if let Some(artifact_ref) = artifact_ref_if_exists(run_dir, run_id, relative_path)? {
            artifacts.push(RunArtifactRef {
                name: name.to_string(),
                artifact_ref,
                schema_version: schema_version.to_string(),
            });
        }
    }
    Ok(artifacts)
}

pub(crate) fn run_evidence_summary(run_dir: &Path) -> LabResult<RunEvidenceSummary> {
    let run_id = run_id_from_dir(run_dir);
    let target_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let toolchain_inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "toolchain/toolchain_inventory.json")?;
    let tool_qualification_summary_ref =
        artifact_ref_if_exists(run_dir, &run_id, "tools/tool_qualification_summary.json")?;
    let observation_ref = artifact_ref_if_exists(run_dir, &run_id, "observations/observe.json")?;
    let load_result_refs = load_result_artifact_refs(run_dir, &run_id)?;
    let control_result_refs = control_result_artifact_refs(run_dir, &run_id)?;
    let audit_facts = audit_facts(run_dir.join("audit.jsonl"), &run_id)?;

    let mut operations_summary = BTreeMap::new();
    operations_summary.insert(
        OP_INVENTORY.to_string(),
        status_if(target_inventory_ref.is_some()),
    );
    operations_summary.insert(
        OP_TOOLCHAIN_DISCOVERY.to_string(),
        status_if(toolchain_inventory_ref.is_some()),
    );
    operations_summary.insert(
        OP_PASSIVE_OBSERVE.to_string(),
        status_if(observation_ref.is_some()),
    );
    operations_summary.insert(
        OP_BOUNDED_LOAD.to_string(),
        status_if(!load_result_refs.is_empty()),
    );
    operations_summary.insert(
        OP_PRIVILEGED_CONTROL.to_string(),
        status_if(!control_result_refs.is_empty()),
    );
    operations_summary.insert(
        OP_CONTROLLED_OPERATING_POINT.to_string(),
        status_if(experiment_has_completed_trial(
            run_dir.join("experiments/experiment_run.json"),
        )?),
    );
    operations_summary.insert(OP_SUSTAINED_THERMAL.to_string(), STATUS_NOT_RUN.to_string());

    let audit_ref = format!("artifact://lab/runs/{run_id}/audit.jsonl");
    let mut operation_audit_refs = BTreeMap::new();
    for operation in audit_facts.operations {
        operation_audit_refs
            .entry(operation)
            .or_insert(audit_ref.clone());
    }

    Ok(RunEvidenceSummary {
        run_id,
        target_inventory_ref,
        toolchain_inventory_ref,
        tool_qualification_summary_ref,
        observation_ref,
        load_result_refs,
        operations_summary,
        operation_audit_refs,
        audit_event_count: audit_facts.event_count,
        audit_run_id_mismatches: audit_facts.run_id_mismatches,
    })
}

#[derive(Debug)]
struct AuditFacts {
    operations: Vec<String>,
    event_count: usize,
    run_id_mismatches: Vec<String>,
}

fn audit_facts(path: PathBuf, expected_run_id: &str) -> LabResult<AuditFacts> {
    if !path.exists() {
        return Ok(AuditFacts {
            operations: Vec::new(),
            event_count: 0,
            run_id_mismatches: Vec::new(),
        });
    }
    let file = fs::File::open(path)?;
    let mut operations = Vec::new();
    let mut event_count = 0;
    let mut run_id_mismatches = Vec::new();
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        event_count += 1;
        let value: serde_json::Value = serde_json::from_str(&line)?;
        if let Some(operation) = value.get("operation").and_then(|value| value.as_str()) {
            operations.push(operation.to_string());
        }
        if let Some(run_id) = value.get("run_id").and_then(|value| value.as_str()) {
            if run_id != expected_run_id {
                run_id_mismatches.push(format!(
                    "audit event line {} run_id {run_id} does not match manifest run_id {expected_run_id}",
                    index + 1
                ));
            }
        }
    }
    Ok(AuditFacts {
        operations,
        event_count,
        run_id_mismatches,
    })
}

fn status_if(completed: bool) -> String {
    if completed {
        STATUS_COMPLETED.to_string()
    } else {
        STATUS_NOT_RUN.to_string()
    }
}

pub(crate) fn experiment_has_completed_trial(path: PathBuf) -> LabResult<bool> {
    let Some(experiment): Option<ExperimentRun> = read_json_artifact_if_exists(path)? else {
        return Ok(false);
    };
    Ok(experiment
        .trials
        .iter()
        .any(|trial| trial.status == "completed"))
}

fn control_result_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    Ok(collect_artifact_refs(run_dir, run_id)?
        .into_iter()
        .filter(|artifact| artifact.contains("/plans/") && artifact.ends_with(".result.json"))
        .collect())
}

fn run_data_quality(
    run_dir: &Path,
    summary: &RunEvidenceSummary,
    identity: &ReleaseIdentity,
) -> LabResult<RunDataQuality> {
    let mut missing = Vec::new();
    let mut inconsistent = Vec::new();
    let mut notes = Vec::new();

    for (label, present) in [
        (
            "target inventory artifact missing",
            summary.target_inventory_ref.is_some(),
        ),
        (
            "toolchain inventory artifact missing",
            summary.toolchain_inventory_ref.is_some(),
        ),
        (
            "passive observation artifact missing",
            summary.observation_ref.is_some(),
        ),
        (
            "tool qualification summary artifact missing",
            summary.tool_qualification_summary_ref.is_some(),
        ),
    ] {
        if !present {
            missing.push(label.to_string());
        }
    }

    for required in READ_ONLY_REQUIRED_AUDIT_OPS {
        if !summary.operation_audit_refs.contains_key(*required) {
            missing.push(format!("audit event missing for {required}"));
        }
    }
    if !summary.load_result_refs.is_empty()
        && !summary.operation_audit_refs.contains_key("load.cpu")
        && !summary
            .operation_audit_refs
            .contains_key("experiment.trial")
    {
        inconsistent.push(
            "bounded load artifact exists without load.cpu or experiment.trial audit event"
                .to_string(),
        );
    }
    if !summary.audit_run_id_mismatches.is_empty() {
        inconsistent.extend(summary.audit_run_id_mismatches.clone());
    }

    if operation_status(summary, OP_CONTROLLED_OPERATING_POINT) == STATUS_NOT_RUN {
        missing.push("controlled operating point experiment was not run".to_string());
    }
    if operation_status(summary, OP_PRIVILEGED_CONTROL) == STATUS_NOT_RUN {
        missing.push("privileged control operation was not run".to_string());
    }
    if operation_status(summary, OP_SUSTAINED_THERMAL) == STATUS_NOT_RUN {
        missing.push("sustained thermal and recovery envelope were not measured".to_string());
    }
    missing.push("wakeups were not measured".to_string());

    if !summary.load_result_refs.is_empty() {
        notes.push("bounded non-root CPU load short-smoke was run".to_string());
    }

    if identity.release_asset_sha256 == "unknown" {
        missing.push("release asset sha256 was not recorded".to_string());
    }
    if identity.adc_lab_target_version == "unknown" {
        missing.push("adc-lab-target version was not recorded".to_string());
    }
    if identity.adc_lab_target_version != "unknown"
        && identity.adc_lab_target_version != identity.adc_lab_version
    {
        inconsistent.push(format!(
            "adc-lab version {} does not match adc-lab-target version {}",
            identity.adc_lab_version, identity.adc_lab_target_version
        ));
    }
    if identity.adc_lab_target_git_sha != "unknown"
        && identity.adc_lab_target_git_sha != identity.adc_lab_git_sha
    {
        inconsistent.push(format!(
            "adc-lab git_sha {} does not match adc-lab-target git_sha {}",
            identity.adc_lab_git_sha, identity.adc_lab_target_git_sha
        ));
    }

    if release_manifest_version_mismatch(run_dir, identity)? {
        inconsistent.push(
            "release manifest version/git_sha does not match adc-lab binary build identity"
                .to_string(),
        );
    }

    Ok(RunDataQuality {
        missing,
        inconsistent,
        notes,
    })
}

fn operation_status<'a>(summary: &'a RunEvidenceSummary, operation: &str) -> &'a str {
    summary
        .operations_summary
        .get(operation)
        .map(String::as_str)
        .unwrap_or(STATUS_NOT_RUN)
}

#[derive(Debug, Clone)]
struct ReleaseIdentity {
    adc_lab_version: String,
    adc_lab_git_sha: String,
    adc_lab_target_version: String,
    adc_lab_target_git_sha: String,
    release_tag: String,
    release_asset: String,
    release_asset_sha256: String,
    binary_sha256: BTreeMap<String, String>,
}

fn release_identity(
    run_dir: &Path,
    controller_build_info: &BuildInfo,
) -> LabResult<ReleaseIdentity> {
    let target_build_info: Option<BuildInfo> =
        read_json_artifact_if_exists(run_dir.join("tools/adc-lab-target.version.json"))?;
    let release_manifest = read_release_manifest(run_dir)?;
    let release_tag = std::env::var("ADC_LAB_RELEASE_TAG")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("v{}", controller_build_info.version));
    let release_asset = std::env::var("ADC_LAB_RELEASE_ASSET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            format!(
                "adc-lab-v{}-{}.tar.gz",
                controller_build_info.version,
                asset_triple(&controller_build_info.target_triple)
            )
        });
    let release_asset_sha256 = std::env::var("ADC_LAB_RELEASE_ASSET_SHA256")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let mut binary_sha256 = BTreeMap::new();
    binary_sha256.insert(
        "adc-lab".to_string(),
        current_exe_sha256().unwrap_or_else(|| "unknown".to_string()),
    );
    if let Some(manifest) = release_manifest.as_ref() {
        for binary in &manifest.binaries {
            binary_sha256.insert(binary.name.clone(), binary.sha256.clone());
        }
    }
    binary_sha256
        .entry("adc-lab-target".to_string())
        .or_insert_with(|| "unknown".to_string());

    Ok(ReleaseIdentity {
        adc_lab_version: controller_build_info.version.clone(),
        adc_lab_git_sha: controller_build_info.git_sha.clone(),
        adc_lab_target_version: target_build_info
            .as_ref()
            .map(|info| info.version.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        adc_lab_target_git_sha: target_build_info
            .as_ref()
            .map(|info| info.git_sha.clone())
            .unwrap_or_else(|| "unknown".to_string()),
        release_tag,
        release_asset,
        release_asset_sha256,
        binary_sha256,
    })
}

fn read_release_manifest(run_dir: &Path) -> LabResult<Option<ReleaseManifest>> {
    let mut candidates = Vec::new();
    if let Ok(path) = std::env::var("ADC_LAB_RELEASE_MANIFEST") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }
    candidates.push(run_dir.join("release-manifest.json"));
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent().and_then(Path::parent) {
            candidates.push(parent.join("release-manifest.json"));
        }
    }

    for candidate in candidates {
        if candidate.exists() {
            return read_json_artifact_if_exists(candidate);
        }
    }
    Ok(None)
}

fn release_manifest_version_mismatch(
    run_dir: &Path,
    identity: &ReleaseIdentity,
) -> LabResult<bool> {
    let Some(manifest) = read_release_manifest(run_dir)? else {
        return Ok(false);
    };
    Ok(
        manifest.version != identity.adc_lab_version
            || manifest.git_sha != identity.adc_lab_git_sha,
    )
}

fn current_exe_sha256() -> Option<String> {
    let path = std::env::current_exe().ok()?;
    sha256_file(&path).ok()
}

fn sha256_file(path: &Path) -> LabResult<String> {
    let bytes = fs::read(path)?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn asset_triple(target_triple: &str) -> String {
    match target_triple {
        "aarch64-unknown-linux-gnu" => "linux-aarch64".to_string(),
        "x86_64-unknown-linux-gnu" => "linux-x86_64".to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn pack_status(summary: &RunEvidenceSummary) -> String {
    let has_core = summary.target_inventory_ref.is_some()
        && summary.toolchain_inventory_ref.is_some()
        && summary.observation_ref.is_some();
    if !has_core {
        "incomplete".to_string()
    } else if operation_status(summary, OP_CONTROLLED_OPERATING_POINT) == STATUS_COMPLETED {
        "controlled_operating_point_subset".to_string()
    } else if operation_status(summary, OP_BOUNDED_LOAD) == STATUS_COMPLETED {
        "exploratory_short_smoke".to_string()
    } else {
        "observational_read_only".to_string()
    }
}

fn run_mode(summary: &RunEvidenceSummary) -> String {
    if operation_status(summary, OP_BOUNDED_LOAD) == STATUS_COMPLETED {
        "exploratory_short_smoke".to_string()
    } else {
        "read_only_familiarization".to_string()
    }
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) -> LabResult<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() {
        return Err(crate::LabError::Validation(format!(
            "artifact collection refuses symlink: {}",
            path.display()
        )));
    }
    if metadata.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        collect_files(&entry.path(), out)?;
    }
    Ok(())
}

pub(crate) fn restore_status(run_dir: &Path) -> LabResult<String> {
    let lease_dir = run_dir.join("leases");
    if !lease_dir.exists() {
        return Ok("not_required".to_string());
    }
    let has_lease = fs::read_dir(lease_dir)?.flatten().next().is_some();
    Ok(if has_lease {
        "pending_or_recorded".to_string()
    } else {
        "not_required".to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn run_manifest_uses_v2_report_run_ref() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        fs::create_dir_all(run_dir.join("reports")).unwrap();
        fs::write(run_dir.join("reports/run_report.v2.json"), "{}").unwrap();
        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert!(manifest
            .artifacts
            .iter()
            .all(|artifact| artifact.artifact_ref.starts_with("artifact://lab/runs/")));
        assert_eq!(
            manifest.claim_trace_ref.as_deref(),
            Some("artifact://lab/runs/LAB-RUN-001/reports/run_report.v2.json")
        );
        assert!(manifest.artifacts.iter().any(|artifact| {
            artifact.name == "report_run" && artifact.schema_version == "lab.artifact.v2"
        }));
        assert!(manifest
            .data_quality
            .missing
            .contains(&"controlled operating point experiment was not run".to_string()));
    }

    #[test]
    fn load_artifact_updates_manifest_summary() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        fs::create_dir_all(run_dir.join("load")).unwrap();
        fs::write(
            run_dir.join("load/cpu.LOAD-RESULT-001.v2.json"),
            serde_json::json!({
                "schema": "lab.artifact.v2",
                "kind": "load",
                "id": "LOAD-RESULT-001",
                "run_id": "LAB-RUN-001",
                "target_id": "local-target",
                "status": {"state": "measured"},
                "payload": {
                    "source_schema_version": "lab.load_result.v1",
                    "load_id": "LOAD-001",
                    "status": "completed",
                    "abort_reason": null,
                    "workers": 2,
                    "duration_ms": 60000,
                    "max_observed_temp_c": 54.5,
                    "operator_abort_observed": false,
                    "safety_monitor_samples": 600,
                    "thermal_surface_available": true,
                    "restore_on_abort_status": "not_required",
                    "worker_iterations": [1, 2]
                },
                "bounds": null,
                "factors": {"controlled": [], "observed": [], "confounders": []},
                "metrics": [],
                "evidence_refs": [],
                "claims": [],
                "data_quality": {"level": "complete", "notes": []},
                "time_unix_ms": 3
            })
            .to_string(),
        )
        .unwrap();
        fs::write(
            run_dir.join("audit.jsonl"),
            [
                audit_line("LAB-RUN-001", "inventory"),
                audit_line("LAB-RUN-001", "toolchain.discover"),
                audit_line("LAB-RUN-001", "tool.qualify_inventory"),
                audit_line("LAB-RUN-001", "observe"),
                audit_line("LAB-RUN-001", "load.cpu"),
            ]
            .join("\n"),
        )
        .unwrap();
        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert_eq!(
            manifest
                .operations_summary
                .get(OP_BOUNDED_LOAD)
                .map(String::as_str),
            Some(STATUS_COMPLETED)
        );
        assert!(!manifest
            .data_quality
            .missing
            .iter()
            .any(|item| item.contains("no load") || item.contains("load or stress")));
        assert!(manifest
            .data_quality
            .notes
            .contains(&"bounded non-root CPU load short-smoke was run".to_string()));
    }

    #[test]
    fn contract_validation_audit_run_id_mismatch_degrades_manifest_quality() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        create_completed_core_artifacts(&run_dir);
        fs::write(
            run_dir.join("audit.jsonl"),
            [
                audit_line("LAB-RUN-OTHER", "inventory"),
                audit_line("LAB-RUN-001", "toolchain.discover"),
                audit_line("LAB-RUN-001", "tool.qualify_inventory"),
                audit_line("LAB-RUN-001", "observe"),
            ]
            .join("\n"),
        )
        .unwrap();

        let manifest = run_manifest(
            &run_dir,
            "local-target".to_string(),
            "local".to_string(),
            1,
            2,
            test_build_info("adc-lab"),
        )
        .unwrap();
        assert!(manifest
            .data_quality
            .inconsistent
            .iter()
            .any(|item| { item.contains("LAB-RUN-OTHER") && item.contains("LAB-RUN-001") }));
    }

    fn target_inventory_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": "lab.target_inventory.v1",
            "target_id": "pi4-target55",
            "target": "ssh://pi4-demo",
            "collected_by": "adc-lab",
            "time_unix_ms": 1780000000000u64,
            "software_stack": {
                "os": "linux",
                "kernel": "6.x",
                "arch": "aarch64",
                "board": "raspberry_pi_4"
            },
            "hardware": {
                "cpu_count": 4,
                "memory_total_kb": 1024,
                "thermal_zones": 1,
                "cpufreq_policies": 1
            },
            "control_surfaces": []
        })
    }

    fn create_completed_core_artifacts(run_dir: &Path) {
        fs::create_dir_all(run_dir.join("inventory")).unwrap();
        fs::create_dir_all(run_dir.join("toolchain")).unwrap();
        fs::create_dir_all(run_dir.join("observations")).unwrap();
        fs::create_dir_all(run_dir.join("tools")).unwrap();
        fs::write(
            run_dir.join("inventory/target_inventory.json"),
            target_inventory_json().to_string(),
        )
        .unwrap();
        fs::write(run_dir.join("toolchain/toolchain_inventory.json"), "{}").unwrap();
        fs::write(run_dir.join("observations/observe.json"), "{}").unwrap();
        fs::write(run_dir.join("tools/tool_qualification_summary.json"), "{}").unwrap();
        fs::write(
            run_dir.join("tools/adc-lab-target.version.json"),
            serde_json::to_string(&test_build_info("adc-lab-target")).unwrap(),
        )
        .unwrap();
    }

    fn audit_line(run_id: &str, operation: &str) -> String {
        serde_json::json!({
            "schema_version": "lab.audit_event.v1",
            "event_id": "EVT-001",
            "run_id": run_id,
            "target_id": "local-target",
            "actor": { "kind": "agent", "id": "codex" },
            "operation": operation,
            "operation_id": null,
            "risk_tier": "tier0_read_only_observation",
            "approval_ref": null,
            "restore_lease_ref": null,
            "result": "recorded",
            "policy_version": "default-lab-policy-v1",
            "time_unix_ms": 1
        })
        .to_string()
    }

    fn test_build_info(name: &str) -> BuildInfo {
        BuildInfo {
            name: name.to_string(),
            version: "0.1.10".to_string(),
            git_sha: "test-git-sha".to_string(),
            target_triple: "x86_64-unknown-linux-gnu".to_string(),
            build_profile: "test".to_string(),
        }
    }
}
