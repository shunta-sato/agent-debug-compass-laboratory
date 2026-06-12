use adc_lab_core::*;
use schemars::{schema_for, JsonSchema};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas/generated"));
    fs::create_dir_all(&out_dir)?;

    write_schema::<Artifact<serde_json::Value>>(&out_dir, "lab.artifact.v2.schema.json")?;
    write_schema::<ClaimDefinition>(&out_dir, "lab.claim_catalog_entry.v2.schema.json")?;
    write_schema::<Artifact<OperatingContractPayload>>(
        &out_dir,
        "lab.report.operating_contract.v2.schema.json",
    )?;
    write_schema::<Artifact<RunReportPayload>>(&out_dir, "lab.report.run.v2.schema.json")?;
    write_schema::<Artifact<RunValidationPayload>>(
        &out_dir,
        "lab.report.run_validation.v2.schema.json",
    )?;
    write_schema::<Artifact<SuitabilityPayload>>(
        &out_dir,
        "lab.report.suitability.v2.schema.json",
    )?;
    write_schema::<Artifact<ConstraintsPayload>>(
        &out_dir,
        "lab.report.constraints.v2.schema.json",
    )?;
    write_schema::<Artifact<ConstraintCheckPayload>>(
        &out_dir,
        "lab.report.constraints_check.v2.schema.json",
    )?;
    write_schema::<Artifact<ObservationPayload>>(&out_dir, "lab.observation.v2.schema.json")?;
    write_schema::<Artifact<LoadPayload>>(&out_dir, "lab.load.v2.schema.json")?;
    write_schema::<Artifact<PressurePayload>>(&out_dir, "lab.pressure.v2.schema.json")?;
    write_schema::<Artifact<CompositePayload>>(&out_dir, "lab.composite.v2.schema.json")?;
    write_schema::<Artifact<WorkloadPayload>>(&out_dir, "lab.workload.v2.schema.json")?;

    write_schema::<ApprovalRecord>(&out_dir, "lab.approval_record.v1.schema.json")?;
    write_schema::<AuditEvent>(&out_dir, "lab.audit_event.v1.schema.json")?;
    write_schema::<BuildInfo>(&out_dir, "lab.build_info.v1.schema.json")?;
    write_schema::<CompositeBoundaryResult>(
        &out_dir,
        "lab.composite_boundary_result.v1.schema.json",
    )?;
    write_schema::<Artifact<GovernorSweepPolicyPayload>>(
        &out_dir,
        "lab.control.governor_sweep_policy.v2.schema.json",
    )?;
    write_schema::<ControlPlan>(&out_dir, "lab.control_plan.v1.schema.json")?;
    write_schema::<ControlResult>(&out_dir, "lab.control_result.v1.schema.json")?;
    write_schema::<ControlSurfaceInventory>(&out_dir, "lab.control_surface.v1.schema.json")?;
    write_schema::<ExperimentMatrix>(&out_dir, "lab.experiment_matrix.v1.schema.json")?;
    write_schema::<ExperimentRun>(&out_dir, "lab.experiment_run.v1.schema.json")?;
    write_schema::<HealthCheck>(&out_dir, "lab.health_check.v1.schema.json")?;
    write_schema::<LoadPlan>(&out_dir, "lab.load_plan.v1.schema.json")?;
    write_schema::<LoadResult>(&out_dir, "lab.load_result.v1.schema.json")?;
    write_schema::<ObservationResult>(&out_dir, "lab.observation_result.v1.schema.json")?;
    write_schema::<PrivilegeDoctorReport>(&out_dir, "lab.privilege_doctor.v1.schema.json")?;
    write_schema::<PrivilegeProviderStatus>(
        &out_dir,
        "lab.privilege_provider_status.v1.schema.json",
    )?;
    write_schema::<PrivilegeSetupPlan>(&out_dir, "lab.privilege_setup_plan.v1.schema.json")?;
    write_schema::<ReleaseManifest>(&out_dir, "lab.release_manifest.v1.schema.json")?;
    write_schema::<ResourcePressureResult>(
        &out_dir,
        "lab.resource_pressure_result.v1.schema.json",
    )?;
    write_schema::<RestoreLease>(&out_dir, "lab.restore_lease.v1.schema.json")?;
    write_schema::<RunContextArtifact>(&out_dir, "lab.run_context.v1.schema.json")?;
    write_schema::<RunManifest>(&out_dir, "lab.run_manifest.v1.schema.json")?;
    write_schema::<SuitabilityPolicy>(&out_dir, "lab.suitability_policy.v1.schema.json")?;
    write_schema::<TargetInventory>(&out_dir, "lab.target_inventory.v1.schema.json")?;
    write_schema::<ToolQualification>(&out_dir, "lab.tool_qualification.v1.schema.json")?;
    write_schema::<ToolQualificationSummary>(
        &out_dir,
        "lab.tool_qualification_summary.v1.schema.json",
    )?;
    write_schema::<ToolchainInventory>(&out_dir, "lab.toolchain_inventory.v1.schema.json")?;
    write_schema::<WorkloadDemandProfile>(&out_dir, "lab.workload_demand_profile.v1.schema.json")?;
    write_schema::<WorkloadFixtureResult>(&out_dir, "lab.workload_fixture_result.v1.schema.json")?;
    write_schema::<WorkloadProfile>(&out_dir, "lab.workload_profile.v1.schema.json")?;
    write_schema::<WorkloadRunPlan>(&out_dir, "lab.workload_run_plan.v1.schema.json")?;
    write_schema::<WorkloadRunResult>(&out_dir, "lab.workload_run_result.v1.schema.json")?;

    Ok(())
}

fn write_schema<T: JsonSchema>(
    out_dir: &Path,
    file_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let schema = schema_for!(T);
    fs::write(out_dir.join(file_name), serde_json::to_vec_pretty(&schema)?)?;
    Ok(())
}
