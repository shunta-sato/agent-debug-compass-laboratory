use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum LabError {
    #[error("io error at {path:?}: {source}")]
    IoWithPath {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    #[error("invalid target: {0}")]
    InvalidTarget(String),
    #[error("invalid duration: {0}")]
    InvalidDuration(String),
    #[error("policy refused operation: {0}")]
    Policy(String),
    #[error("missing required surface: {0}")]
    MissingSurface(String),
    #[error("external command failed: {0}")]
    Command(String),
    #[error("validation failed: {0}")]
    Validation(String),
}

pub type LabResult<T> = Result<T, LabError>;

pub trait IoPathExt<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> LabResult<T>;
}

impl<T> IoPathExt<T> for std::io::Result<T> {
    fn with_path(self, path: impl Into<PathBuf>) -> LabResult<T> {
        self.map_err(|source| LabError::IoWithPath {
            path: path.into(),
            source,
        })
    }
}
