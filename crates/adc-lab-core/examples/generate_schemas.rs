use adc_lab_core::{Artifact, ClaimDefinition, OperatingContractPayload, SuitabilityPayload};
use schemars::schema_for;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas/generated"));
    fs::create_dir_all(&out_dir)?;

    let artifact_schema = schema_for!(Artifact<serde_json::Value>);
    fs::write(
        out_dir.join("lab.artifact.v2.schema.json"),
        serde_json::to_vec_pretty(&artifact_schema)?,
    )?;

    let claim_schema = schema_for!(ClaimDefinition);
    fs::write(
        out_dir.join("lab.claim_catalog_entry.v2.schema.json"),
        serde_json::to_vec_pretty(&claim_schema)?,
    )?;

    let operating_contract_schema = schema_for!(Artifact<OperatingContractPayload>);
    fs::write(
        out_dir.join("lab.report.operating_contract.v2.schema.json"),
        serde_json::to_vec_pretty(&operating_contract_schema)?,
    )?;

    let suitability_schema = schema_for!(Artifact<SuitabilityPayload>);
    fs::write(
        out_dir.join("lab.report.suitability.v2.schema.json"),
        serde_json::to_vec_pretty(&suitability_schema)?,
    )?;

    Ok(())
}
