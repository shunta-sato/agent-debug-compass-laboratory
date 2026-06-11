use adc_lab_core::{
    Artifact, ClaimDefinition, CompositePayload, LoadPayload, ObservationPayload,
    OperatingContractPayload, PressurePayload, RunReportPayload, SuitabilityPayload,
    WorkloadPayload,
};
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

    let run_report_schema = schema_for!(Artifact<RunReportPayload>);
    fs::write(
        out_dir.join("lab.report.run.v2.schema.json"),
        serde_json::to_vec_pretty(&run_report_schema)?,
    )?;

    let suitability_schema = schema_for!(Artifact<SuitabilityPayload>);
    fs::write(
        out_dir.join("lab.report.suitability.v2.schema.json"),
        serde_json::to_vec_pretty(&suitability_schema)?,
    )?;

    let observation_schema = schema_for!(Artifact<ObservationPayload>);
    fs::write(
        out_dir.join("lab.observation.v2.schema.json"),
        serde_json::to_vec_pretty(&observation_schema)?,
    )?;

    let load_schema = schema_for!(Artifact<LoadPayload>);
    fs::write(
        out_dir.join("lab.load.v2.schema.json"),
        serde_json::to_vec_pretty(&load_schema)?,
    )?;

    let pressure_schema = schema_for!(Artifact<PressurePayload>);
    fs::write(
        out_dir.join("lab.pressure.v2.schema.json"),
        serde_json::to_vec_pretty(&pressure_schema)?,
    )?;

    let composite_schema = schema_for!(Artifact<CompositePayload>);
    fs::write(
        out_dir.join("lab.composite.v2.schema.json"),
        serde_json::to_vec_pretty(&composite_schema)?,
    )?;

    let workload_schema = schema_for!(Artifact<WorkloadPayload>);
    fs::write(
        out_dir.join("lab.workload.v2.schema.json"),
        serde_json::to_vec_pretty(&workload_schema)?,
    )?;

    Ok(())
}
