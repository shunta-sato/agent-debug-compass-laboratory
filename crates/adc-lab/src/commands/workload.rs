use super::super::*;
use super::common::*;
use adc_lab_core::ids::now_unix_ms;
use anyhow::Context;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub(crate) fn command_workload_run(args: WorkloadRunCommand) -> Result<()> {
    let target = TargetSpec::parse(&args.target)?;
    let run = create_or_open_run(args.run_dir)?;
    let plan = read_workload_plan_for_command(&args.plan)?;
    let target_id = args
        .target_id
        .clone()
        .unwrap_or_else(|| target.target_id.clone());
    let artifacts = match target.transport {
        TargetTransport::Local => {
            let workload_dir = run
                .run_dir
                .join("workloads")
                .join(safe_artifact_id(&plan.workload_id, "workload"));
            fs::create_dir_all(&workload_dir)
                .with_context(|| format!("failed to create {}", workload_dir.display()))?;
            let stdout_path = workload_dir.join("stdout.txt");
            let stderr_path = workload_dir.join("stderr.txt");
            run_local_workload(
                &plan,
                &LocalWorkloadRunOptions {
                    run_id: run.run_id.clone(),
                    target_id,
                    execution_mode: args.execution_mode.into(),
                    stdout_path,
                    stderr_path,
                },
            )?
        }
        TargetTransport::Ssh => refused_workload_artifacts(
            run.run_id.clone(),
            plan.workload_id.clone(),
            target_id,
            "remote_workload_execution_not_supported_in_v1".to_string(),
        ),
    };
    persist_workload_artifacts(&run, &plan, artifacts)
}

fn persist_workload_artifacts(
    run: &RunContext,
    plan: &WorkloadRunPlan,
    mut artifacts: LocalWorkloadRunArtifacts,
) -> Result<()> {
    let workload_dir = run
        .run_dir
        .join("workloads")
        .join(safe_artifact_id(&plan.workload_id, "workload"));
    fs::create_dir_all(&workload_dir)
        .with_context(|| format!("failed to create {}", workload_dir.display()))?;
    let plan_path = workload_dir.join("workload_run_plan.json");
    let result_path = workload_dir.join("workload_run_result.json");
    let profile_path = run.run_dir.join("reports/workload_demand_profile.json");
    let stdout_path = workload_dir.join("stdout.txt");
    let stderr_path = workload_dir.join("stderr.txt");
    let plan_ref = write_json_artifact(run, &plan_path, plan)?;
    let stdout_ref = stdout_path
        .exists()
        .then(|| run.artifact_uri(&stdout_path))
        .transpose()?;
    let stderr_ref = stderr_path
        .exists()
        .then(|| run.artifact_uri(&stderr_path))
        .transpose()?;
    artifacts.result.stdout_ref = stdout_ref;
    artifacts.result.stderr_ref = stderr_ref;
    let result_ref = write_json_artifact(run, &result_path, &artifacts.result)?;
    artifacts.demand_profile.evidence_refs.push(plan_ref);
    artifacts
        .demand_profile
        .evidence_refs
        .push(result_ref.clone());
    artifacts.demand_profile.evidence_refs.sort();
    artifacts.demand_profile.evidence_refs.dedup();
    let profile_ref = write_json_artifact(run, &profile_path, &artifacts.demand_profile)?;
    let mut store = evidence_store_for_run(run)?;
    write_workload_artifact_v2(&mut store, &run.run_dir, artifacts.demand_profile.clone())?;
    append_audit_event(
        run,
        AuditInput {
            target_id: artifacts.result.target_id.clone(),
            actor: Actor::codex(),
            operation: "workload.run".to_string(),
            operation_id: Some(plan.workload_id.clone()),
            risk_tier: RiskTier::Tier1LowRiskReversibleNonRoot,
            approval_ref: None,
            restore_lease_ref: None,
            result: serde_json::to_string(&artifacts.result.status)
                .unwrap_or_else(|_| "unknown".to_string())
                .trim_matches('"')
                .to_string(),
        },
    )?;
    artifacts
        .result
        .audit_refs
        .push(run.artifact_uri(run.run_dir.join("audit.jsonl"))?);
    write_json_artifact(run, &result_path, &artifacts.result)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&ArtifactOutput {
            artifact_ref: profile_ref,
            value: artifacts.demand_profile,
        })?
    );
    Ok(())
}

pub(crate) fn command_workload_fixture_bounded_smoke(args: BoundedSmokeCommand) -> Result<()> {
    if args.duration_ms == 0 || args.duration_ms > 60_000 {
        anyhow::bail!("bounded-smoke duration_ms must be 1..=60000");
    }
    if args.memory_bytes > 256 * 1024 * 1024 {
        anyhow::bail!("bounded-smoke memory_bytes must be <= 256MiB");
    }
    if args.storage_bytes > 64 * 1024 * 1024 {
        anyhow::bail!("bounded-smoke storage_bytes must be <= 64MiB");
    }
    let started = Instant::now();
    let mut memory = vec![0u8; args.memory_bytes as usize];
    for index in (0..memory.len()).step_by(4096) {
        memory[index] = (index as u8).wrapping_add(1);
    }
    let storage_dir = args.storage_dir.unwrap_or_else(std::env::temp_dir);
    let storage_path = storage_dir.join(format!("adc-lab-workload-smoke-{}.tmp", now_unix_ms()));
    let mut written = 0u64;
    if args.storage_bytes > 0 {
        let mut file = fs::File::create(&storage_path)
            .with_context(|| format!("failed to create {}", storage_path.display()))?;
        let chunk = vec![0x5au8; 8192.min(args.storage_bytes as usize)];
        while written < args.storage_bytes {
            let to_write = (args.storage_bytes - written).min(chunk.len() as u64) as usize;
            file.write_all(&chunk[..to_write])?;
            written += to_write as u64;
        }
        file.sync_data()?;
        drop(file);
        let mut file = fs::File::open(&storage_path)
            .with_context(|| format!("failed to open {}", storage_path.display()))?;
        let mut buf = [0u8; 8192];
        while file.read(&mut buf)? != 0 {}
        fs::remove_file(&storage_path)
            .with_context(|| format!("failed to remove {}", storage_path.display()))?;
    }
    let mut iterations = 0u64;
    while started.elapsed() < Duration::from_millis(args.duration_ms) {
        iterations = iterations.wrapping_add(1);
        std::hint::black_box(iterations.rotate_left(7).wrapping_mul(0x9e37_79b9));
    }
    print_json(&WorkloadFixtureResult {
        schema_version: "lab.workload_fixture_result.v1".to_string(),
        fixture: "bounded_smoke".to_string(),
        duration_ms: started.elapsed().as_millis() as u64,
        memory_bytes_touched: args.memory_bytes,
        storage_bytes_written_and_cleaned: written,
        iterations,
        claim_boundary: vec![
            "exploratory target-local capability evidence only".to_string(),
            "not real application performance".to_string(),
            "not production readiness".to_string(),
            "not sustained thermal safety".to_string(),
            "not flash-wear evidence".to_string(),
        ],
    })
}

fn read_workload_plan_for_command(path: &Path) -> Result<WorkloadRunPlan> {
    let plan: WorkloadRunPlan = read_yaml(path)?;
    Ok(plan)
}
