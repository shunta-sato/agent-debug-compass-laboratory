use crate::contracts::{
    LoadResult, ObservedCapabilityResults, RunManifest, TargetCapabilityProfile,
    TargetCapabilityStatus, WorkloadClass, WorkloadProfile,
};
use crate::ids::now_unix_ms;
use crate::observe::ObservationResult;
use crate::{artifact_uri_for_run, run_id_from_run_dir, LabError, LabResult};
use serde::de::DeserializeOwned;
use std::fs;
use std::path::{Path, PathBuf};

pub fn target_capability_profile(
    run_dir: impl AsRef<Path>,
    target_id: String,
    workload: WorkloadProfile,
) -> LabResult<TargetCapabilityProfile> {
    validate_workload_profile(&workload)?;
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_dir(run_dir);
    let evidence_pack_ref = artifact_ref_if_exists(run_dir, &run_id, "run_manifest.json")?;
    let manifest: Option<RunManifest> =
        read_json_artifact_if_exists(run_dir.join("run_manifest.json"))?;
    let load_artifacts = load_result_artifacts(run_dir, &run_id)?;
    let observation_artifacts = observation_artifacts(run_dir, &run_id)?;
    let observed_results = observed_capability_results(&load_artifacts, &observation_artifacts);
    let capability_status = capability_status(&workload, &observed_results);
    let mut evidence_refs = Vec::new();
    evidence_refs.extend(evidence_pack_ref.clone());
    evidence_refs.extend(
        load_artifacts
            .iter()
            .map(|artifact| artifact.artifact_ref.clone()),
    );
    evidence_refs.extend(
        observation_artifacts
            .iter()
            .map(|artifact| artifact.artifact_ref.clone()),
    );
    evidence_refs.sort();
    evidence_refs.dedup();

    Ok(TargetCapabilityProfile {
        schema_version: "lab.target_capability_profile.v1".to_string(),
        target_id,
        workload_id: workload.workload_id,
        evidence_pack_ref,
        capability_status,
        selection_ready: false,
        supported_claims: supported_claims(&observed_results),
        blocked_claims: blocked_claims(manifest.as_ref()),
        next_evidence_needed: next_evidence_needed(&observed_results, manifest.as_ref()),
        observed_results,
        evidence_refs,
        time_unix_ms: now_unix_ms(),
    })
}

fn validate_workload_profile(workload: &WorkloadProfile) -> LabResult<()> {
    if workload.schema_version != "lab.workload_profile.v1" {
        return Err(LabError::Validation(format!(
            "unsupported workload profile schema_version {}",
            workload.schema_version
        )));
    }
    if workload.workload_id.trim().is_empty() {
        return Err(LabError::Validation(
            "workload_id must be non-empty".to_string(),
        ));
    }
    if workload.description.trim().is_empty() {
        return Err(LabError::Validation(
            "workload description must be non-empty".to_string(),
        ));
    }
    if workload.duration_seconds == 0 {
        return Err(LabError::Validation(
            "workload duration_seconds must be > 0".to_string(),
        ));
    }
    if workload.measurement_requirements.is_empty() {
        return Err(LabError::Validation(
            "workload measurement_requirements must be non-empty".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
struct LoadArtifact {
    artifact_ref: String,
    result: LoadResult,
}

#[derive(Debug)]
struct ObservationArtifact {
    artifact_ref: String,
    result: ObservationResult,
}

fn observed_capability_results(
    load_artifacts: &[LoadArtifact],
    observation_artifacts: &[ObservationArtifact],
) -> ObservedCapabilityResults {
    let mut max_temp_c = load_artifacts
        .iter()
        .filter_map(|artifact| artifact.result.max_observed_temp_c)
        .fold(None, max_option_f64);

    let mut frequency_values = Vec::new();
    let mut memory_available_kb_min = None;
    let mut observation_sample_count = 0u64;

    for artifact in observation_artifacts {
        observation_sample_count += artifact.result.samples.len() as u64;
        for sample in &artifact.result.samples {
            if let Some(temp) = sample.max_temp_c {
                max_temp_c = max_option_f64(max_temp_c, temp);
            }
            if let Some(freq) = sample.avg_cpu_freq_khz {
                frequency_values.push(freq);
            }
            if let Some(memory) = sample.memory_available_kb {
                memory_available_kb_min = Some(
                    memory_available_kb_min.map_or(memory, |current: u64| current.min(memory)),
                );
            }
        }
    }

    let cpu_frequency_range_khz = if let (Some(min), Some(max)) =
        (frequency_values.iter().min(), frequency_values.iter().max())
    {
        Some(vec![*min, *max])
    } else {
        None
    };

    ObservedCapabilityResults {
        max_temp_c,
        abort_count: load_artifacts
            .iter()
            .filter(|artifact| {
                artifact.result.abort_reason.is_some() || artifact.result.status != "completed"
            })
            .count() as u64,
        cpu_frequency_range_khz,
        memory_available_kb_min,
        max_load_duration_ms: load_artifacts
            .iter()
            .map(|artifact| artifact.result.duration_ms)
            .max()
            .unwrap_or(0),
        max_observation_duration_ms: observation_artifacts
            .iter()
            .map(|artifact| artifact.result.duration_ms)
            .max()
            .unwrap_or(0),
        load_result_count: load_artifacts.len() as u64,
        observation_sample_count,
    }
}

fn max_option_f64(current: Option<f64>, candidate: f64) -> Option<f64> {
    Some(current.map_or(candidate, |value| value.max(candidate)))
}

fn capability_status(
    workload: &WorkloadProfile,
    observed: &ObservedCapabilityResults,
) -> TargetCapabilityStatus {
    let has_load = observed.load_result_count > 0;
    let has_observation = observed.observation_sample_count > 0;
    if !has_load && !has_observation {
        return TargetCapabilityStatus::NoEvidence;
    }
    if requirements_met_for_short_smoke(workload, observed) {
        TargetCapabilityStatus::MeasuredShortSmoke
    } else if has_load || has_observation {
        TargetCapabilityStatus::ExploratoryPartial
    } else {
        TargetCapabilityStatus::InsufficientForSelection
    }
}

fn requirements_met_for_short_smoke(
    workload: &WorkloadProfile,
    observed: &ObservedCapabilityResults,
) -> bool {
    let required_duration_ms = workload.duration_seconds.saturating_mul(1000);
    let duration_met = match workload.workload_class {
        WorkloadClass::SyntheticCpu => observed.max_load_duration_ms >= required_duration_ms,
        WorkloadClass::IdleObserve => observed.max_observation_duration_ms >= required_duration_ms,
        WorkloadClass::ApplicationWorkload => {
            observed
                .max_load_duration_ms
                .max(observed.max_observation_duration_ms)
                >= required_duration_ms
        }
    };
    if !duration_met {
        return false;
    }
    if let Some(max_abort_count) = workload.requirements.max_abort_count {
        if observed.abort_count > max_abort_count {
            return false;
        }
        if observed.load_result_count == 0 {
            return false;
        }
    }
    if let Some(max_temp) = workload.requirements.thermal_celsius_max {
        let Some(observed_temp) = observed.max_temp_c else {
            return false;
        };
        if observed_temp > max_temp {
            return false;
        }
    }
    if workload.requirements.memory_mb_max.is_some()
        || workload.requirements.latency_p95_ms_max.is_some()
    {
        return false;
    }
    observed.load_result_count > 0 || observed.observation_sample_count > 0
}

fn supported_claims(observed: &ObservedCapabilityResults) -> Vec<String> {
    let mut claims = Vec::new();
    if observed.observation_sample_count > 0 {
        claims.push(
            "target produced passive observation samples under the current target policy"
                .to_string(),
        );
    }
    if observed.load_result_count > 0 && observed.abort_count == 0 {
        claims.push(
            "target completed bounded workload short-smoke artifact(s) without recorded abort"
                .to_string(),
        );
    } else if observed.load_result_count > 0 {
        claims.push(
            "target produced bounded workload result artifact(s), but aborts block completion claims"
                .to_string(),
        );
    }
    claims
}

fn blocked_claims(manifest: Option<&RunManifest>) -> Vec<String> {
    let mut claims = vec![
        "Pi4 is sufficient for this workload".to_string(),
        "Pi5 is required for this workload".to_string(),
        "battery safe".to_string(),
        "sustained production ready".to_string(),
        "all operating points measured".to_string(),
        "fixed-frequency behavior verified".to_string(),
        "low overhead under all operating points".to_string(),
    ];
    if manifest_has_identity_inconsistency(manifest) {
        claims.push(
            "formal comparison requires matching adc-lab and adc-lab-target release identity"
                .to_string(),
        );
    }
    claims
}

fn next_evidence_needed(
    observed: &ObservedCapabilityResults,
    manifest: Option<&RunManifest>,
) -> Vec<String> {
    let mut needed = Vec::new();
    if observed.observation_sample_count == 0 {
        needed.push("passive observation artifact for the workload run".to_string());
    }
    if observed.load_result_count == 0 {
        needed.push("bounded workload result artifact".to_string());
    }
    needed.extend([
        "Pi4/Pi5 same-suite target comparison".to_string(),
        "30 minute sustained thermal run".to_string(),
        "controlled operating point sweep".to_string(),
        "latency/jitter measurement".to_string(),
        "battery/power and storage/write evidence".to_string(),
    ]);
    if manifest_has_identity_inconsistency(manifest) {
        needed.push("rerun with matching release binary identity and checksums".to_string());
    }
    needed
}

fn manifest_has_identity_inconsistency(manifest: Option<&RunManifest>) -> bool {
    manifest
        .map(|manifest| {
            manifest.data_quality.inconsistent.iter().any(|item| {
                item.contains("adc-lab version")
                    || item.contains("adc-lab git_sha")
                    || item.contains("release manifest version")
                    || item.contains("run_id")
            })
        })
        .unwrap_or(false)
}

fn load_result_artifacts(run_dir: &Path, run_id: &str) -> LabResult<Vec<LoadArtifact>> {
    collect_run_files(run_dir)?
        .into_iter()
        .filter(|path| is_load_result_path(run_dir, path))
        .map(|path| {
            let artifact_ref = artifact_uri_for_run(run_id, run_dir, &path)?;
            let result = read_json_artifact(&path)?;
            Ok(LoadArtifact {
                artifact_ref,
                result,
            })
        })
        .collect()
}

fn observation_artifacts(run_dir: &Path, run_id: &str) -> LabResult<Vec<ObservationArtifact>> {
    collect_run_files(run_dir)?
        .into_iter()
        .filter(|path| is_observation_path(run_dir, path))
        .map(|path| {
            let artifact_ref = artifact_uri_for_run(run_id, run_dir, &path)?;
            let result = read_json_artifact(&path)?;
            Ok(ObservationArtifact {
                artifact_ref,
                result,
            })
        })
        .collect()
}

fn is_load_result_path(run_dir: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(run_dir) else {
        return false;
    };
    let relative = relative.to_string_lossy();
    relative == "loads/load_result.json"
        || (relative.starts_with("loads/") && relative.ends_with(".result.json"))
        || (relative.starts_with("experiments/trials/") && relative.ends_with("/load_result.json"))
}

fn is_observation_path(run_dir: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(run_dir) else {
        return false;
    };
    let relative = relative.to_string_lossy();
    relative == "observations/observe.json"
        || (relative.starts_with("experiments/trials/") && relative.ends_with("/observation.json"))
}

fn read_json_artifact<T: DeserializeOwned>(path: &Path) -> LabResult<T> {
    let bytes = fs::read(path)?;
    serde_json::from_slice(&bytes).map_err(|error| {
        LabError::Validation(format!(
            "failed to parse JSON artifact {}: {error}",
            path.display()
        ))
    })
}

fn read_json_artifact_if_exists<T: DeserializeOwned>(path: PathBuf) -> LabResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    read_json_artifact(&path).map(Some)
}

fn artifact_ref_if_exists(
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

fn collect_run_files(run_dir: &Path) -> LabResult<Vec<PathBuf>> {
    let mut files = Vec::new();
    collect_files(run_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files(path: &Path, out: &mut Vec<PathBuf>) -> LabResult<()> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }
    if !path.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        collect_files(&entry?.path(), out)?;
    }
    Ok(())
}

fn run_id_from_dir(run_dir: &Path) -> String {
    run_id_from_run_dir(run_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{
        LoadRestoreOnAbortStatus, LoadSafetyMonitorResult, WorkloadClaimBoundary, WorkloadClass,
        WorkloadRequirements,
    };
    use crate::fsutil::write_json_pretty;
    use crate::observe::{ObservationSample, Signal};

    #[test]
    fn capability_profile_blocks_selection_claims() {
        let temp = tempfile::tempdir().unwrap();
        let workload = workload();
        let profile =
            target_capability_profile(temp.path(), "pi4-demo".to_string(), workload).unwrap();
        assert!(!profile.selection_ready);
        assert_eq!(
            profile.capability_status,
            TargetCapabilityStatus::NoEvidence
        );
        assert!(profile
            .blocked_claims
            .iter()
            .any(|claim| claim == "Pi4 is sufficient for this workload"));
        assert!(profile
            .blocked_claims
            .iter()
            .any(|claim| claim == "Pi5 is required for this workload"));
    }

    #[test]
    fn capability_profile_extracts_observe_and_load_results() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("observations")).unwrap();
        std::fs::create_dir_all(temp.path().join("loads")).unwrap();
        write_json_pretty(
            temp.path().join("run_manifest.json"),
            &valid_run_manifest_json(),
        )
        .unwrap();
        write_json_pretty(
            temp.path().join("observations/observe.json"),
            &ObservationResult {
                schema_version: "lab.observation_result.v1".to_string(),
                target_id: "pi4-demo".to_string(),
                duration_ms: 1_000,
                signals: vec![Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory],
                samples: vec![
                    ObservationSample {
                        sample_index: 0,
                        cpu_total_ticks: Some(10),
                        cpu_idle_ticks: Some(9),
                        memory_available_kb: Some(1_024_000),
                        avg_cpu_freq_khz: Some(600_000),
                        max_temp_c: Some(45.0),
                    },
                    ObservationSample {
                        sample_index: 1,
                        cpu_total_ticks: Some(20),
                        cpu_idle_ticks: Some(18),
                        memory_available_kb: Some(1_000_000),
                        avg_cpu_freq_khz: Some(1_800_000),
                        max_temp_c: Some(54.0),
                    },
                ],
            },
        )
        .unwrap();
        write_json_pretty(
            temp.path().join("loads/LOAD-001.result.json"),
            &LoadResult {
                schema_version: "lab.load_result.v1".to_string(),
                result_id: "LOAD-RESULT-001".to_string(),
                load_id: "LOAD-001".to_string(),
                target_id: "pi4-demo".to_string(),
                status: "completed".to_string(),
                workers: 2,
                duration_ms: 60_000,
                abort_reason: None,
                max_observed_temp_c: Some(55.0),
                worker_iterations: vec![10, 11],
                safety_monitor: LoadSafetyMonitorResult {
                    sample_interval_ms: 100,
                    samples: 10,
                    thermal_surface_available: true,
                    operator_abort_observed: false,
                    restore_on_abort_status: LoadRestoreOnAbortStatus::NotRequired,
                },
                time_unix_ms: 1,
            },
        )
        .unwrap();

        let profile =
            target_capability_profile(temp.path(), "pi4-demo".to_string(), workload()).unwrap();
        assert_eq!(
            profile.capability_status,
            TargetCapabilityStatus::MeasuredShortSmoke
        );
        assert_eq!(profile.observed_results.max_temp_c, Some(55.0));
        assert_eq!(profile.observed_results.abort_count, 0);
        assert_eq!(
            profile.observed_results.cpu_frequency_range_khz,
            Some(vec![600_000, 1_800_000])
        );
        assert_eq!(
            profile.observed_results.memory_available_kb_min,
            Some(1_000_000)
        );
        assert_eq!(profile.observed_results.max_load_duration_ms, 60_000);
        assert_eq!(profile.observed_results.max_observation_duration_ms, 1_000);
        assert_eq!(profile.observed_results.load_result_count, 1);
        assert_eq!(profile.observed_results.observation_sample_count, 2);
        assert!(profile
            .evidence_refs
            .iter()
            .all(|artifact| artifact.starts_with("artifact://lab/runs/")));
    }

    #[test]
    fn capability_profile_blocks_formal_comparison_when_toolchain_identity_is_inconsistent() {
        let temp = tempfile::tempdir().unwrap();
        let mut manifest = valid_run_manifest_json();
        manifest["data_quality"]["inconsistent"] = serde_json::json!([
            "adc-lab version 0.1.10 does not match adc-lab-target version 0.1.9"
        ]);
        write_json_pretty(temp.path().join("run_manifest.json"), &manifest).unwrap();

        let profile =
            target_capability_profile(temp.path(), "pi4-demo".to_string(), workload()).unwrap();
        assert!(!profile.selection_ready);
        assert!(profile.blocked_claims.iter().any(|claim| {
            claim.contains("formal comparison requires matching adc-lab and adc-lab-target")
        }));
        assert!(profile
            .next_evidence_needed
            .iter()
            .any(|item| { item.contains("matching release binary identity") }));
    }

    #[test]
    fn capability_profile_rejects_bad_workload_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let mut workload = workload();
        workload.schema_version = "bad.schema".to_string();
        let error =
            target_capability_profile(temp.path(), "pi4-demo".to_string(), workload).unwrap_err();
        assert!(error.to_string().contains("schema_version"));
    }

    fn workload() -> WorkloadProfile {
        WorkloadProfile {
            schema_version: "lab.workload_profile.v1".to_string(),
            workload_id: "bounded_cpu_load_2_workers_60s".to_string(),
            description: "Bounded CPU load with 2 workers for 60 seconds".to_string(),
            workload_class: WorkloadClass::SyntheticCpu,
            duration_seconds: 60,
            requirements: WorkloadRequirements {
                thermal_celsius_max: Some(75.0),
                max_abort_count: Some(0),
                memory_mb_max: None,
                latency_p95_ms_max: None,
            },
            measurement_requirements: vec![
                "cpu_busy_percent".to_string(),
                "temperature_celsius".to_string(),
                "frequency_khz".to_string(),
                "memory_available_kb".to_string(),
            ],
            claim_boundary: WorkloadClaimBoundary::SyntheticShortSmokeOnly,
        }
    }

    fn valid_run_manifest_json() -> serde_json::Value {
        serde_json::json!({
            "schema_version": "lab.run_manifest.v1",
            "run_id": "LAB-RUN-001",
            "target_id": "pi4-demo",
            "target": "local",
            "mode": "exploratory_short_smoke",
            "started_at_unix_ms": 1,
            "ended_at_unix_ms": 2,
            "adc_lab_version": "0.1.10",
            "adc_lab_git_sha": "test",
            "adc_lab_target_version": "0.1.10",
            "adc_lab_target_git_sha": "test",
            "release_tag": "v0.1.10",
            "release_asset": "adc-lab-v0.1.10-linux-x86_64.tar.gz",
            "release_asset_sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "binary_sha256": {
                "adc-lab": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "adc-lab-target": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
            },
            "operations_summary": {
                "inventory": "completed",
                "toolchain_discovery": "completed",
                "passive_observe": "completed",
                "bounded_load": "completed",
                "privileged_control": "not_run",
                "controlled_operating_point": "not_run",
                "sustained_thermal": "not_run"
            },
            "operation_audit_refs": {},
            "artifacts": [],
            "audit_ref": "artifact://lab/runs/LAB-RUN-001/audit.jsonl",
            "claim_trace_ref": null,
            "data_quality": {
                "missing": [],
                "inconsistent": [],
                "notes": []
            }
        })
    }
}
