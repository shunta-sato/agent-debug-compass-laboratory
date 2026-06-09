use crate::error::{IoPathExt, LabResult};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn read_to_string_lossy(path: impl AsRef<Path>) -> LabResult<Option<String>> {
    let path = path.as_ref();
    match fs::read(path) {
        Ok(bytes) => Ok(Some(
            String::from_utf8_lossy(&bytes)
                .trim_matches(char::from(0))
                .trim()
                .to_string(),
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(crate::LabError::IoWithPath {
            path: path.to_path_buf(),
            source: err,
        }),
    }
}

pub fn read_json<T: DeserializeOwned>(path: impl AsRef<Path>) -> LabResult<T> {
    let path = path.as_ref();
    let bytes = fs::read(path).with_path(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn read_yaml<T: DeserializeOwned>(path: impl AsRef<Path>) -> LabResult<T> {
    let path = path.as_ref();
    let bytes = fs::read(path).with_path(path)?;
    Ok(serde_yaml::from_slice(&bytes)?)
}

pub fn write_json_pretty<T: Serialize>(path: impl AsRef<Path>, value: &T) -> LabResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_path(path)
}

pub fn append_json_line<T: Serialize>(path: impl AsRef<Path>, value: &T) -> LabResult<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_path(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_path(path)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n").with_path(path)?;
    Ok(())
}

pub fn ensure_dir(path: impl AsRef<Path>) -> LabResult<()> {
    let path = path.as_ref();
    fs::create_dir_all(path).with_path(path)
}
