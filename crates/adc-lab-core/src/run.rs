use crate::error::{IoPathExt, LabResult};
use crate::ids::{new_id, now_unix_ms};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RunContext {
    pub run_id: String,
    pub run_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunContextArtifact {
    schema_version: String,
    run_id: String,
}

impl RunContext {
    pub fn artifact_uri(&self, path: impl AsRef<Path>) -> LabResult<String> {
        artifact_uri_for_run(&self.run_id, &self.run_dir, path)
    }
}

pub fn artifact_uri_for_run(
    run_id: &str,
    run_dir: impl AsRef<Path>,
    path: impl AsRef<Path>,
) -> LabResult<String> {
    let run_dir = run_dir.as_ref();
    let path = path.as_ref();
    reject_symlink_components(run_dir, path)?;
    let relative = path.strip_prefix(run_dir).map_err(|_| {
        crate::LabError::Validation("artifact path escapes run directory".to_string())
    })?;
    let relative = relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    if relative.is_empty() || relative.starts_with("..") || relative.contains("/../") {
        return Err(crate::LabError::Validation(
            "artifact path escapes run directory".to_string(),
        ));
    }
    Ok(format!("artifact://lab/runs/{run_id}/{relative}"))
}

fn reject_symlink_components(run_dir: &Path, path: &Path) -> LabResult<()> {
    let run_meta = std::fs::symlink_metadata(run_dir).with_path(run_dir)?;
    if run_meta.file_type().is_symlink() {
        return Err(crate::LabError::Validation(
            "run directory symlink is not allowed in artifact refs".to_string(),
        ));
    }
    let relative = path.strip_prefix(run_dir).map_err(|_| {
        crate::LabError::Validation("artifact path escapes run directory".to_string())
    })?;
    let mut current = run_dir.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&current).with_path(&current)?;
        if metadata.file_type().is_symlink() {
            return Err(crate::LabError::Validation(format!(
                "artifact path contains symlink component: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

pub fn create_or_open_run(run_dir: Option<PathBuf>) -> LabResult<RunContext> {
    let (run_id, run_dir) = match run_dir {
        Some(path) => {
            let run_id = existing_run_id(&path)?.unwrap_or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| name.starts_with("LAB-RUN-"))
                    .map(ToString::to_string)
                    .unwrap_or_else(|| new_id("LAB-RUN"))
            });
            (run_id, path)
        }
        None => {
            let run_id = format!("LAB-RUN-{}", now_unix_ms());
            (
                run_id.clone(),
                PathBuf::from("lab").join("runs").join(run_id),
            )
        }
    };

    for child in [
        "inventory",
        "toolchain",
        "observations",
        "plans",
        "approvals",
        "leases",
        "loads",
        "workloads",
        "pressure",
        "decisions",
        "constraints",
        "experiments",
        "reports",
        "tools",
        "privilege",
    ] {
        let path = run_dir.join(child);
        std::fs::create_dir_all(&path).with_path(path)?;
    }

    write_run_context_if_missing(&run_dir, &run_id)?;
    Ok(RunContext { run_id, run_dir })
}

pub fn run_id_from_run_dir(run_dir: &Path) -> String {
    existing_run_id(run_dir)
        .ok()
        .flatten()
        .or_else(|| {
            run_dir
                .file_name()
                .and_then(|name| name.to_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| "LAB-RUN-unknown".to_string())
}

fn existing_run_id(run_dir: &Path) -> LabResult<Option<String>> {
    let context_path = run_dir.join("run_context.json");
    if context_path.exists() {
        let bytes = std::fs::read(&context_path).with_path(&context_path)?;
        let context: RunContextArtifact = serde_json::from_slice(&bytes)?;
        if !context.run_id.trim().is_empty() {
            return Ok(Some(context.run_id));
        }
    }
    let manifest_path = run_dir.join("run_manifest.json");
    if manifest_path.exists() {
        let bytes = std::fs::read(&manifest_path).with_path(&manifest_path)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes)?;
        if let Some(run_id) = value.get("run_id").and_then(|value| value.as_str()) {
            if !run_id.trim().is_empty() {
                return Ok(Some(run_id.to_string()));
            }
        }
    }
    Ok(None)
}

fn write_run_context_if_missing(run_dir: &Path, run_id: &str) -> LabResult<()> {
    let path = run_dir.join("run_context.json");
    if path.exists() {
        return Ok(());
    }
    let context = RunContextArtifact {
        schema_version: "lab.run_context.v1".to_string(),
        run_id: run_id.to_string(),
    };
    let bytes = serde_json::to_vec_pretty(&context)?;
    std::fs::write(&path, bytes).with_path(path)?;
    Ok(())
}
