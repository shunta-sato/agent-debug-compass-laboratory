use crate::contracts::{CapabilityCostModel, FamiliarizationPack, OperatingPointCoverage};
use crate::ids::now_unix_ms;
use crate::{artifact_uri_for_run, LabResult};
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub fn pack_run(run_dir: impl AsRef<Path>, target_id: String) -> LabResult<FamiliarizationPack> {
    let run_dir = run_dir.as_ref();
    let run_id = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("LAB-RUN-unknown")
        .to_string();
    let artifact_refs = collect_artifact_refs(run_dir, &run_id)?;
    let audit_event_count = count_audit_events(run_dir.join("audit.jsonl"))?;
    let restore_status = restore_status(run_dir)?;
    let claim_trace_ref = artifact_refs
        .iter()
        .find(|path| path.ends_with("claim_evidence_trace.json"))
        .cloned();
    Ok(FamiliarizationPack {
        schema_version: "lab.familiarization_pack.v1".to_string(),
        run_id,
        target_id,
        artifact_refs,
        audit_event_count,
        restore_status,
        claim_trace_ref,
        time_unix_ms: now_unix_ms(),
    })
}

pub fn operating_point_coverage(run_id: String, target_id: String) -> OperatingPointCoverage {
    OperatingPointCoverage {
        schema_version: "lab.operating_point_coverage.v1".to_string(),
        run_id,
        target_id,
        covered_points: vec!["default_dynamic_policy_observed_or_planned".to_string()],
        blocked_points: vec!["all_fixed_cpu_frequencies".to_string()],
        coverage_status: "provisional".to_string(),
        time_unix_ms: now_unix_ms(),
    }
}

pub fn capability_cost_model(run_id: String, target_id: String) -> CapabilityCostModel {
    let (cost_model_status, limitations) = if target_id == "local-target" {
        (
            "provisional_host_fallback_only".to_string(),
            vec![
                "host fallback cannot prove Pi4/Pi5 physical footprint".to_string(),
                "target characterization is required before production budget claims".to_string(),
            ],
        )
    } else {
        (
            "provisional_short_target_smoke".to_string(),
            vec![
                "short bounded target smoke does not calibrate production NFR budgets".to_string(),
                "longer operating-envelope and degraded-mode runs are required before production budget claims".to_string(),
            ],
        )
    };
    CapabilityCostModel {
        schema_version: "lab.capability_cost_model.v1".to_string(),
        run_id,
        target_id,
        capabilities: vec!["cpu_load_response".to_string(), "thermal_trend".to_string()],
        cost_model_status,
        limitations,
        time_unix_ms: now_unix_ms(),
    }
}

fn collect_artifact_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    let mut paths = Vec::new();
    collect_files(run_dir, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| artifact_uri_for_run(run_id, run_dir, path))
        .collect()
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

fn count_audit_events(path: PathBuf) -> LabResult<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let file = fs::File::open(path)?;
    Ok(std::io::BufReader::new(file).lines().count())
}

fn restore_status(run_dir: &Path) -> LabResult<String> {
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

    #[test]
    fn contract_validation_pack_uses_logical_artifact_refs() {
        let temp = tempfile::tempdir().unwrap();
        let run_dir = temp.path().join("LAB-RUN-001");
        let report_dir = run_dir.join("reports");
        fs::create_dir_all(&report_dir).unwrap();
        fs::write(run_dir.join("audit.jsonl"), "{}\n").unwrap();
        fs::write(report_dir.join("claim_evidence_trace.json"), "{}").unwrap();

        let pack = pack_run(&run_dir, "local-target".to_string()).unwrap();
        assert!(pack
            .artifact_refs
            .iter()
            .all(|artifact| artifact.starts_with("artifact://lab/runs/LAB-RUN-001/")));
        assert!(pack
            .artifact_refs
            .iter()
            .all(|artifact| !artifact.contains(temp.path().to_str().unwrap())));
    }
}
