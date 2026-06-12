use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ARTIFACT_SCHEMA_V2: &str = "lab.artifact.v2";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Inventory,
    Observation,
    Pressure,
    Composite,
    ControlResult,
    Load,
    Workload,
    #[serde(rename = "report.run")]
    ReportRun,
    #[serde(rename = "report.operating_contract")]
    ReportOperatingContract,
    #[serde(rename = "report.suitability")]
    ReportSuitability,
    #[serde(rename = "report.constraints")]
    ReportConstraints,
    #[serde(rename = "report.constraints_check")]
    ReportConstraintsCheck,
    #[serde(rename = "report.run_validation")]
    ReportRunValidation,
    #[serde(rename = "control.governor_sweep_policy")]
    ControlGovernorSweepPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "state", content = "detail", rename_all = "snake_case")]
pub enum Status {
    Measured,
    MeasuredPartial,
    Insufficient,
    NotApplicable {
        reason: String,
    },
    Refused {
        code: EvidenceRefusalCode,
        message: String,
    },
    UnsafeBlocked {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceRefusalCode {
    InvalidArtifact,
    PolicyViolation,
    UnsupportedOperation,
    MissingEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Decision {
    Supported,
    Provisional,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    pub decision: Decision,
    pub evidence_refs: Vec<String>,
    pub next_evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub duration_seconds_max: Option<u64>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields)]
pub struct Factors {
    pub controlled: Vec<String>,
    pub observed: Vec<String>,
    pub confounders: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataQualityLevel {
    Complete,
    Partial,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DataQuality {
    pub level: DataQualityLevel,
    pub notes: Vec<String>,
}

impl Default for DataQuality {
    fn default() -> Self {
        Self {
            level: DataQualityLevel::Complete,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Artifact<P> {
    pub schema: String,
    pub kind: Kind,
    pub id: String,
    pub run_id: String,
    pub target_id: String,
    pub status: Status,
    pub bounds: Option<Bounds>,
    pub factors: Factors,
    pub metrics: Vec<Metric>,
    pub claims: Vec<Claim>,
    pub evidence_refs: Vec<String>,
    pub data_quality: DataQuality,
    pub payload: P,
    pub time_unix_ms: u64,
}

impl<P> Artifact<P> {
    pub fn new(
        kind: Kind,
        id: impl Into<String>,
        run_id: impl Into<String>,
        target_id: impl Into<String>,
        status: Status,
        payload: P,
        time_unix_ms: u64,
    ) -> Self {
        Self {
            schema: ARTIFACT_SCHEMA_V2.to_string(),
            kind,
            id: id.into(),
            run_id: run_id.into(),
            target_id: target_id.into(),
            status,
            bounds: None,
            factors: Factors::default(),
            metrics: Vec::new(),
            claims: Vec::new(),
            evidence_refs: Vec::new(),
            data_quality: DataQuality::default(),
            payload,
            time_unix_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ArtifactHeader {
    pub schema: String,
    pub kind: Kind,
    pub id: String,
    pub run_id: String,
    pub target_id: String,
}
