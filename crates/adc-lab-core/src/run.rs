use crate::error::{IoPathExt, LabResult};
use crate::ids::{new_id, now_unix_ms};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct RunContext {
    pub run_id: String,
    pub run_dir: PathBuf,
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
            let run_id = path
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| name.starts_with("LAB-RUN-"))
                .map(ToString::to_string)
                .unwrap_or_else(|| new_id("LAB-RUN"));
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
        "experiments",
        "reports",
        "tools",
    ] {
        let path = run_dir.join(child);
        std::fs::create_dir_all(&path).with_path(path)?;
    }

    Ok(RunContext { run_id, run_dir })
}
