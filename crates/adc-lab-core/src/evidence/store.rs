use crate::error::{IoPathExt, LabResult};
use crate::evidence::envelope::{ArtifactHeader, ARTIFACT_SCHEMA_V2};
use crate::evidence::{Artifact, Kind};
use crate::fsutil::{append_json_line, write_json_pretty};
use crate::ids::{new_id, now_unix_ms};
use crate::run::artifact_uri_for_run;
use crate::runfs::artifact_path;
use crate::{Actor, AuditEvent, RiskTier, POLICY_VERSION};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMeta {
    pub path: PathBuf,
    pub artifact_ref: String,
    pub kind: Kind,
    pub id: String,
    pub run_id: String,
    pub target_id: String,
}

#[derive(Debug, Clone)]
pub struct EvidenceStore {
    run_dirs: Vec<PathBuf>,
    artifacts: Vec<ArtifactMeta>,
    by_kind: BTreeMap<String, Vec<usize>>,
}

impl EvidenceStore {
    pub fn open(run_dirs: &[PathBuf]) -> LabResult<Self> {
        let mut store = Self {
            run_dirs: run_dirs.to_vec(),
            artifacts: Vec::new(),
            by_kind: BTreeMap::new(),
        };
        for run_dir in run_dirs {
            reject_symlink(run_dir)?;
            store.scan_run_dir(run_dir, run_dir)?;
        }
        Ok(store)
    }

    pub fn run_dirs(&self) -> &[PathBuf] {
        &self.run_dirs
    }

    pub fn iter(&self, kind: Kind) -> impl Iterator<Item = &ArtifactMeta> {
        let key = kind_key(&kind);
        self.by_kind
            .get(&key)
            .into_iter()
            .flat_map(|indexes| indexes.iter())
            .map(|index| &self.artifacts[*index])
    }

    pub fn all(&self) -> &[ArtifactMeta] {
        &self.artifacts
    }

    pub fn load<P: DeserializeOwned>(&self, meta: &ArtifactMeta) -> LabResult<Artifact<P>> {
        let bytes = fs::read(&meta.path).with_path(&meta.path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn write<P: Serialize>(
        &mut self,
        run_dir: &Path,
        relative_path: &Path,
        artifact: &Artifact<P>,
    ) -> LabResult<String> {
        reject_symlink(run_dir)?;
        let path = artifact_path(run_dir, relative_path)?;
        write_json_pretty(&path, artifact)?;
        let artifact_ref = artifact_uri_for_run(&artifact.run_id, run_dir, &path)?;
        let meta = ArtifactMeta {
            path: path.clone(),
            artifact_ref: artifact_ref.clone(),
            kind: artifact.kind,
            id: artifact.id.clone(),
            run_id: artifact.run_id.clone(),
            target_id: artifact.target_id.clone(),
        };
        self.insert(meta);
        append_json_line(
            run_dir.join("audit.jsonl"),
            &AuditEvent {
                schema_version: "lab.audit_event.v1".to_string(),
                event_id: new_id("EVT"),
                run_id: artifact.run_id.clone(),
                target_id: artifact.target_id.clone(),
                actor: Actor::codex(),
                operation: "evidence.write".to_string(),
                operation_id: Some(artifact.id.clone()),
                risk_tier: RiskTier::Tier0ReadOnlyObservation,
                approval_ref: None,
                restore_lease_ref: None,
                result: serde_json::to_string(&artifact.status)
                    .unwrap_or_else(|_| "unknown".to_string())
                    .trim_matches('"')
                    .to_string(),
                policy_version: POLICY_VERSION.to_string(),
                time_unix_ms: now_unix_ms(),
            },
        )?;
        Ok(artifact_ref)
    }

    fn scan_run_dir(&mut self, run_dir: &Path, current: &Path) -> LabResult<()> {
        for entry in fs::read_dir(current).with_path(current)? {
            let entry = entry.with_path(current)?;
            let path = entry.path();
            reject_symlink(&path)?;
            if path.is_dir() {
                self.scan_run_dir(run_dir, &path)?;
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
                self.index_json_if_v2(run_dir, &path)?;
            }
        }
        Ok(())
    }

    fn index_json_if_v2(&mut self, run_dir: &Path, path: &Path) -> LabResult<()> {
        let bytes = fs::read(path).with_path(path)?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|source| {
            crate::LabError::Validation(format!(
                "malformed JSON artifact at {}: {source}",
                path.display()
            ))
        })?;
        let Some(schema) = value.get("schema").and_then(|schema| schema.as_str()) else {
            return Ok(());
        };
        if schema != ARTIFACT_SCHEMA_V2 {
            return Ok(());
        }
        let header: ArtifactHeader = serde_json::from_value(value)?;
        let artifact_ref = artifact_uri_for_run(&header.run_id, run_dir, path)?;
        self.insert(ArtifactMeta {
            path: path.to_path_buf(),
            artifact_ref,
            kind: header.kind,
            id: header.id,
            run_id: header.run_id,
            target_id: header.target_id,
        });
        Ok(())
    }

    fn insert(&mut self, meta: ArtifactMeta) {
        let key = kind_key(&meta.kind);
        let index = self.artifacts.len();
        self.artifacts.push(meta);
        self.by_kind.entry(key).or_default().push(index);
    }
}

fn kind_key(kind: &Kind) -> String {
    serde_json::to_string(kind).unwrap_or_else(|_| format!("{kind:?}"))
}

fn reject_symlink(path: &Path) -> LabResult<()> {
    let metadata = fs::symlink_metadata(path).with_path(path)?;
    if metadata.file_type().is_symlink() {
        return Err(crate::LabError::Validation(format!(
            "evidence store refuses symlink path: {}",
            path.display()
        )));
    }
    Ok(())
}
