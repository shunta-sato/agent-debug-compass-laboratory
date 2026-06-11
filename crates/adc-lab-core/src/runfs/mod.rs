use crate::error::LabResult;
use std::path::{Component, Path, PathBuf};

pub fn artifact_path(
    run_dir: impl AsRef<Path>,
    relative_path: impl AsRef<Path>,
) -> LabResult<PathBuf> {
    let run_dir = run_dir.as_ref();
    let relative_path = relative_path.as_ref();
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(crate::LabError::Validation(
            "artifact relative path must stay inside run directory".to_string(),
        ));
    }
    Ok(run_dir.join(relative_path))
}
