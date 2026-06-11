use crate::contracts::{
    WorkloadDataQuality, WorkloadDemand, WorkloadDemandProfile, WorkloadDemandScope,
    WorkloadExecutionMode, WorkloadRunPlan, WorkloadRunResult, WorkloadRunStatus,
    WorkloadSystemContext, WorkloadTargetConditionedResponse,
};
use crate::error::IoPathExt;
use crate::fsutil::read_to_string_lossy;
use crate::ids::{new_id, now_unix_ms};
use crate::observe::max_temp_c;
use crate::{LabError, LabResult};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const MAX_WORKLOAD_DURATION_SECONDS: u64 = 300;
const ASSUMED_CLK_TCK: f64 = 100.0;

#[derive(Debug, Clone)]
pub struct LocalWorkloadRunOptions {
    pub run_id: String,
    pub target_id: String,
    pub execution_mode: WorkloadExecutionMode,
    pub stdout_path: PathBuf,
    pub stderr_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LocalWorkloadRunArtifacts {
    pub result: WorkloadRunResult,
    pub demand_profile: WorkloadDemandProfile,
}

#[derive(Debug, Clone, Default)]
struct ProcSnapshot {
    utime_ticks: Option<u64>,
    stime_ticks: Option<u64>,
    rss_kb: Option<u64>,
    vmhwm_kb: Option<u64>,
    read_bytes: Option<u64>,
    write_bytes: Option<u64>,
    cancelled_write_bytes: Option<u64>,
    voluntary_ctxt_switches: Option<u64>,
    nonvoluntary_ctxt_switches: Option<u64>,
}

#[derive(Debug, Clone)]
struct WorkloadSample {
    elapsed_ms: u64,
    proc: Option<ProcSnapshot>,
    temp_c: Option<f64>,
    freq_khz: Option<u64>,
    mem_available_kb: Option<u64>,
    cpu_ticks: Option<(u64, u64)>,
}

pub fn run_local_workload(
    plan: &WorkloadRunPlan,
    options: &LocalWorkloadRunOptions,
) -> LabResult<LocalWorkloadRunArtifacts> {
    validate_workload_plan(plan)?;
    let started_at_unix_ms = now_unix_ms();
    let started = Instant::now();
    let stdout_file = File::create(&options.stdout_path).with_path(&options.stdout_path)?;
    let stderr_file = File::create(&options.stderr_path).with_path(&options.stderr_path)?;
    let mut command = Command::new(&plan.execution.executable_path);
    command
        .args(&plan.execution.args)
        .current_dir(&plan.execution.working_directory)
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    if !plan.execution.environment_policy.inherit {
        command.env_clear();
    }
    for var in &plan.execution.environment_policy.allowed {
        command.env(&var.name, &var.value);
    }
    let mut child = command.spawn().map_err(|source| LabError::IoWithPath {
        path: PathBuf::from(&plan.execution.executable_path),
        source,
    })?;
    let pid = child.id();
    let deadline = started + Duration::from_secs(plan.bounds.duration_seconds_max);
    let sample_interval = Duration::from_millis(plan.observation.sample_interval_ms.max(10));
    let mut samples = Vec::new();
    let mut reason = None;
    let status: WorkloadRunStatus;
    let mut stdout_truncated = false;
    let mut stderr_truncated = false;

    loop {
        samples.push(sample_workload(pid, started.elapsed().as_millis() as u64));
        if let Some(path) = plan.bounds.operator_abort_file.as_deref() {
            if Path::new(path).exists() {
                reason = Some("operator_abort".to_string());
                status = WorkloadRunStatus::Aborted;
                let _ = child.kill();
                break;
            }
        }
        if let Some(limit) = plan.bounds.thermal_abort_c {
            if let Some(temp) = max_temp_c() {
                if temp >= limit {
                    reason = Some(format!("thermal_abort_at_{temp:.1}c"));
                    status = WorkloadRunStatus::Aborted;
                    let _ = child.kill();
                    break;
                }
            }
        }
        let stdout_len = fs::metadata(&options.stdout_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        let stderr_len = fs::metadata(&options.stderr_path)
            .map(|meta| meta.len())
            .unwrap_or(0);
        if stdout_len > plan.bounds.stdout_bytes_max {
            stdout_truncated = true;
            reason = Some("stdout_limit_exceeded".to_string());
            status = WorkloadRunStatus::Aborted;
            let _ = child.kill();
            break;
        }
        if stderr_len > plan.bounds.stderr_bytes_max {
            stderr_truncated = true;
            reason = Some("stderr_limit_exceeded".to_string());
            status = WorkloadRunStatus::Aborted;
            let _ = child.kill();
            break;
        }
        if let Some(exit_status) = child.try_wait()? {
            let success = exit_status.success();
            status = if success {
                WorkloadRunStatus::Completed
            } else {
                WorkloadRunStatus::Failed
            };
            if !success {
                reason = Some("process_exit_nonzero".to_string());
            }
            break;
        }
        if Instant::now() >= deadline {
            reason = Some("duration_limit_exceeded".to_string());
            status = WorkloadRunStatus::Aborted;
            let _ = child.kill();
            break;
        }
        std::thread::sleep(sample_interval.min(deadline.saturating_duration_since(Instant::now())));
    }

    let exit = child.wait().ok();
    truncate_if_needed(
        &options.stdout_path,
        plan.bounds.stdout_bytes_max,
        &mut stdout_truncated,
    )?;
    truncate_if_needed(
        &options.stderr_path,
        plan.bounds.stderr_bytes_max,
        &mut stderr_truncated,
    )?;
    samples.push(sample_workload(pid, started.elapsed().as_millis() as u64));
    let ended_at_unix_ms = now_unix_ms();
    let duration_ms = started.elapsed().as_millis() as u64;
    let exit_code = exit.as_ref().and_then(|status| status.code());
    #[cfg(unix)]
    let signal = exit.as_ref().and_then(|status| status.signal());
    #[cfg(not(unix))]
    let signal = None;
    let mut data_quality = WorkloadDataQuality {
        degraded: !matches!(status, WorkloadRunStatus::Completed),
        missing: Vec::new(),
        notes: vec![
            "process CPU time assumes CLK_TCK=100 for v1 Linux targets".to_string(),
            "child process demand is not aggregated in v1".to_string(),
        ],
    };
    if samples.iter().all(|sample| sample.proc.is_none()) {
        data_quality
            .missing
            .push("process-scoped demand unavailable from /proc/<pid>".to_string());
    }
    if !matches!(status, WorkloadRunStatus::Completed) {
        data_quality
            .notes
            .push("workload demand profile was generated from incomplete run evidence".to_string());
    }
    let demand_scope = if samples.iter().any(|sample| sample.proc.is_some()) {
        WorkloadDemandScope::ProcessScoped
    } else {
        WorkloadDemandScope::SystemWideOnly
    };
    let demand = summarize_demand(&samples, duration_ms);
    let response = summarize_response(&samples, plan.bounds.thermal_abort_c, reason.clone());
    let system_context = summarize_system_context(&samples);
    let result = WorkloadRunResult {
        schema_version: "lab.workload_run_result.v1".to_string(),
        run_id: options.run_id.clone(),
        workload_id: plan.workload_id.clone(),
        target_id: options.target_id.clone(),
        execution_mode: options.execution_mode.clone(),
        status,
        reason: reason.clone(),
        exit_code,
        signal,
        started_at_unix_ms,
        ended_at_unix_ms,
        duration_ms,
        stdout_ref: None,
        stderr_ref: None,
        stdout_truncated,
        stderr_truncated,
        process_ids: vec![pid],
        audit_refs: Vec::new(),
        data_quality: data_quality.clone(),
        time_unix_ms: now_unix_ms(),
    };
    let profile = WorkloadDemandProfile {
        schema_version: "lab.workload_demand_profile.v1".to_string(),
        profile_id: new_id("WORKLOAD-DEMAND"),
        run_id: options.run_id.clone(),
        workload_id: plan.workload_id.clone(),
        target_id: options.target_id.clone(),
        execution_mode: options.execution_mode.clone(),
        demand_scope,
        workload_demand: demand,
        target_conditioned_response: response,
        system_context,
        data_quality,
        evidence_refs: Vec::new(),
        time_unix_ms: now_unix_ms(),
    };
    Ok(LocalWorkloadRunArtifacts {
        result,
        demand_profile: profile,
    })
}

pub fn refused_workload_artifacts(
    run_id: String,
    workload_id: String,
    target_id: String,
    reason: String,
) -> LocalWorkloadRunArtifacts {
    let data_quality = WorkloadDataQuality {
        degraded: true,
        missing: vec!["workload run v1 is local-target only".to_string()],
        notes: vec![reason.clone()],
    };
    let result = WorkloadRunResult {
        schema_version: "lab.workload_run_result.v1".to_string(),
        run_id: run_id.clone(),
        workload_id: workload_id.clone(),
        target_id: target_id.clone(),
        execution_mode: WorkloadExecutionMode::Local,
        status: WorkloadRunStatus::Refused,
        reason: Some(reason.clone()),
        exit_code: None,
        signal: None,
        started_at_unix_ms: now_unix_ms(),
        ended_at_unix_ms: now_unix_ms(),
        duration_ms: 0,
        stdout_ref: None,
        stderr_ref: None,
        stdout_truncated: false,
        stderr_truncated: false,
        process_ids: Vec::new(),
        audit_refs: Vec::new(),
        data_quality: data_quality.clone(),
        time_unix_ms: now_unix_ms(),
    };
    let demand_profile = WorkloadDemandProfile {
        schema_version: "lab.workload_demand_profile.v1".to_string(),
        profile_id: new_id("WORKLOAD-DEMAND"),
        run_id,
        workload_id,
        target_id,
        execution_mode: WorkloadExecutionMode::Local,
        demand_scope: WorkloadDemandScope::SystemWideOnly,
        workload_demand: empty_demand(),
        target_conditioned_response: WorkloadTargetConditionedResponse {
            portable_between_targets: false,
            thermal_max_c: None,
            thermal_margin_c: None,
            freq_range_khz: None,
            abort_reason: Some(reason),
        },
        system_context: WorkloadSystemContext {
            system_cpu_percent_avg: None,
            system_memory_available_min_kb: None,
            background_activity_confounder: "not_measured_refused_run".to_string(),
        },
        data_quality,
        evidence_refs: Vec::new(),
        time_unix_ms: now_unix_ms(),
    };
    LocalWorkloadRunArtifacts {
        result,
        demand_profile,
    }
}

pub fn validate_workload_plan(plan: &WorkloadRunPlan) -> LabResult<()> {
    if plan.schema_version != "lab.workload_run_plan.v1" {
        return Err(LabError::Validation(
            "workload plan schema_version must be lab.workload_run_plan.v1".to_string(),
        ));
    }
    if plan.workload_id.trim().is_empty() {
        return Err(LabError::Validation("workload_id is required".to_string()));
    }
    if plan.bounds.duration_seconds_max == 0
        || plan.bounds.duration_seconds_max > MAX_WORKLOAD_DURATION_SECONDS
    {
        return Err(LabError::Policy(format!(
            "workload duration must be 1..={MAX_WORKLOAD_DURATION_SECONDS}s"
        )));
    }
    if plan.observation.sample_interval_ms == 0 {
        return Err(LabError::Validation(
            "sample_interval_ms must be > 0".to_string(),
        ));
    }
    if !plan.observation.process_scoped {
        return Err(LabError::Policy(
            "workload run v1 requires process-scoped observation".to_string(),
        ));
    }
    reject_shell_execution(plan)?;
    let executable = Path::new(&plan.execution.executable_path);
    if !executable.is_absolute() {
        return Err(LabError::Policy(
            "workload executable_path must be absolute".to_string(),
        ));
    }
    let metadata = fs::metadata(executable).with_path(executable)?;
    if !metadata.is_file() {
        return Err(LabError::Validation(
            "workload executable_path must point to a file".to_string(),
        ));
    }
    #[cfg(unix)]
    {
        let mode = metadata.permissions().mode();
        if plan.execution.reject_setuid && mode & 0o4000 != 0 {
            return Err(LabError::Policy(
                "workload executable setuid bit is refused".to_string(),
            ));
        }
        if plan.execution.reject_world_writable && mode & 0o002 != 0 {
            return Err(LabError::Policy(
                "workload executable world-writable mode is refused".to_string(),
            ));
        }
    }
    let working_directory = Path::new(&plan.execution.working_directory);
    if !working_directory.is_absolute() || !working_directory.is_dir() {
        return Err(LabError::Policy(
            "workload working_directory must be an existing absolute directory".to_string(),
        ));
    }
    if plan.execution.require_executable_sha256
        && plan.execution.expected_executable_sha256.is_none()
    {
        return Err(LabError::Policy(
            "workload executable sha256 is required by plan policy".to_string(),
        ));
    }
    if let Some(expected) = plan.execution.expected_executable_sha256.as_deref() {
        let actual = sha256_file(executable)?;
        if actual != expected {
            return Err(LabError::Policy(
                "workload executable sha256 mismatch".to_string(),
            ));
        }
    }
    Ok(())
}

fn reject_shell_execution(plan: &WorkloadRunPlan) -> LabResult<()> {
    let executable = Path::new(&plan.execution.executable_path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(executable, "sh" | "bash" | "dash" | "zsh" | "fish")
        && plan.execution.args.iter().any(|arg| arg == "-c")
    {
        return Err(LabError::Policy(
            "workload plan must not execute shell command strings".to_string(),
        ));
    }
    Ok(())
}

fn sha256_file(path: &Path) -> LabResult<String> {
    let mut file = File::open(path).with_path(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let read = file.read(&mut buf).with_path(path)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn truncate_if_needed(path: &Path, limit: u64, truncated: &mut bool) -> LabResult<()> {
    let len = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    if len > limit {
        *truncated = true;
        OpenOptions::new()
            .write(true)
            .open(path)
            .with_path(path)?
            .set_len(limit)
            .with_path(path)?;
    }
    Ok(())
}

fn sample_workload(pid: u32, elapsed_ms: u64) -> WorkloadSample {
    WorkloadSample {
        elapsed_ms,
        proc: read_proc_snapshot(pid).ok(),
        temp_c: max_temp_c(),
        freq_khz: avg_cpu_freq_khz(),
        mem_available_kb: mem_available_kb(),
        cpu_ticks: cpu_ticks(),
    }
}

fn read_proc_snapshot(pid: u32) -> LabResult<ProcSnapshot> {
    let proc_dir = PathBuf::from("/proc").join(pid.to_string());
    let stat = read_to_string_lossy(proc_dir.join("stat"))?;
    let status = read_to_string_lossy(proc_dir.join("status"))?;
    let io = read_to_string_lossy(proc_dir.join("io"))?;
    let mut snapshot = ProcSnapshot::default();
    if let Some(stat) = stat {
        let (utime, stime) = parse_proc_stat(&stat)?;
        snapshot.utime_ticks = Some(utime);
        snapshot.stime_ticks = Some(stime);
    }
    if let Some(status) = status {
        parse_proc_status(&status, &mut snapshot);
    }
    if let Some(io) = io {
        parse_proc_io(&io, &mut snapshot);
    }
    if snapshot.utime_ticks.is_none()
        && snapshot.stime_ticks.is_none()
        && snapshot.rss_kb.is_none()
        && snapshot.read_bytes.is_none()
        && snapshot.write_bytes.is_none()
    {
        return Err(LabError::MissingSurface(
            "process-scoped /proc/<pid> snapshot unavailable".to_string(),
        ));
    }
    Ok(snapshot)
}

fn parse_proc_stat(text: &str) -> LabResult<(u64, u64)> {
    let end = text.rfind(") ").ok_or_else(|| {
        LabError::Validation("could not parse /proc/<pid>/stat comm field".to_string())
    })?;
    let fields = text[end + 2..].split_whitespace().collect::<Vec<_>>();
    let utime = fields
        .get(11)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| LabError::Validation("missing utime in /proc/<pid>/stat".to_string()))?;
    let stime = fields
        .get(12)
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| LabError::Validation("missing stime in /proc/<pid>/stat".to_string()))?;
    Ok((utime, stime))
}

fn parse_proc_status(text: &str, snapshot: &mut ProcSnapshot) {
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next().unwrap_or_default().trim_end_matches(':') {
            "VmRSS" => snapshot.rss_kb = parts.next().and_then(|value| value.parse().ok()),
            "VmHWM" => snapshot.vmhwm_kb = parts.next().and_then(|value| value.parse().ok()),
            "voluntary_ctxt_switches" => {
                snapshot.voluntary_ctxt_switches = parts.next().and_then(|value| value.parse().ok())
            }
            "nonvoluntary_ctxt_switches" => {
                snapshot.nonvoluntary_ctxt_switches =
                    parts.next().and_then(|value| value.parse().ok())
            }
            _ => {}
        }
    }
}

fn parse_proc_io(text: &str, snapshot: &mut ProcSnapshot) {
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next().unwrap_or_default().trim_end_matches(':') {
            "read_bytes" => snapshot.read_bytes = parts.next().and_then(|value| value.parse().ok()),
            "write_bytes" => {
                snapshot.write_bytes = parts.next().and_then(|value| value.parse().ok())
            }
            "cancelled_write_bytes" => {
                snapshot.cancelled_write_bytes = parts.next().and_then(|value| value.parse().ok())
            }
            _ => {}
        }
    }
}

fn summarize_demand(samples: &[WorkloadSample], duration_ms: u64) -> WorkloadDemand {
    let proc_samples = samples
        .iter()
        .filter_map(|sample| sample.proc.as_ref())
        .collect::<Vec<_>>();
    let (first, last) = match (proc_samples.first(), proc_samples.last()) {
        (Some(first), Some(last)) => (*first, *last),
        _ => return empty_demand(),
    };
    let utime_delta = delta(first.utime_ticks, last.utime_ticks);
    let stime_delta = delta(first.stime_ticks, last.stime_ticks);
    let total_ticks = opt_sum(utime_delta, stime_delta);
    let process_cpu_time_ms = total_ticks.map(|ticks| ticks as f64 * 1000.0 / ASSUMED_CLK_TCK);
    let process_cpu_percent_avg = process_cpu_time_ms
        .and_then(|cpu_ms| (duration_ms > 0).then_some((cpu_ms / duration_ms as f64) * 100.0));
    let process_cpu_percent_peak = peak_process_cpu_percent(samples);
    WorkloadDemand {
        process_cpu_utime_ticks: utime_delta,
        process_cpu_stime_ticks: stime_delta,
        process_cpu_time_ms,
        process_cpu_percent_avg,
        process_cpu_percent_peak,
        rss_peak_kb: proc_samples.iter().filter_map(|s| s.rss_kb).max(),
        vmhwm_peak_kb: proc_samples.iter().filter_map(|s| s.vmhwm_kb).max(),
        read_bytes: last.read_bytes,
        write_bytes: last.write_bytes,
        cancelled_write_bytes: last.cancelled_write_bytes,
        voluntary_ctxt_switches: last.voluntary_ctxt_switches,
        nonvoluntary_ctxt_switches: last.nonvoluntary_ctxt_switches,
        duty_cycle: "bounded_burst".to_string(),
        child_process_accounting_status: "unsupported".to_string(),
    }
}

fn empty_demand() -> WorkloadDemand {
    WorkloadDemand {
        process_cpu_utime_ticks: None,
        process_cpu_stime_ticks: None,
        process_cpu_time_ms: None,
        process_cpu_percent_avg: None,
        process_cpu_percent_peak: None,
        rss_peak_kb: None,
        vmhwm_peak_kb: None,
        read_bytes: None,
        write_bytes: None,
        cancelled_write_bytes: None,
        voluntary_ctxt_switches: None,
        nonvoluntary_ctxt_switches: None,
        duty_cycle: "unknown".to_string(),
        child_process_accounting_status: "unsupported".to_string(),
    }
}

fn summarize_response(
    samples: &[WorkloadSample],
    thermal_abort_c: Option<f64>,
    abort_reason: Option<String>,
) -> WorkloadTargetConditionedResponse {
    let thermal_max_c = samples
        .iter()
        .filter_map(|sample| sample.temp_c)
        .reduce(f64::max);
    let thermal_margin_c = thermal_abort_c
        .zip(thermal_max_c)
        .map(|(limit, temp)| limit - temp);
    let min_freq = samples.iter().filter_map(|sample| sample.freq_khz).min();
    let max_freq = samples.iter().filter_map(|sample| sample.freq_khz).max();
    WorkloadTargetConditionedResponse {
        portable_between_targets: false,
        thermal_max_c,
        thermal_margin_c,
        freq_range_khz: min_freq.zip(max_freq).map(|(min, max)| vec![min, max]),
        abort_reason,
    }
}

fn summarize_system_context(samples: &[WorkloadSample]) -> WorkloadSystemContext {
    WorkloadSystemContext {
        system_cpu_percent_avg: system_cpu_percent(samples),
        system_memory_available_min_kb: samples.iter().filter_map(|s| s.mem_available_kb).min(),
        background_activity_confounder: "measured_partial".to_string(),
    }
}

fn peak_process_cpu_percent(samples: &[WorkloadSample]) -> Option<f64> {
    let mut peak = None;
    for pair in samples.windows(2) {
        let Some(first) = pair[0].proc.as_ref() else {
            continue;
        };
        let Some(last) = pair[1].proc.as_ref() else {
            continue;
        };
        let elapsed = pair[1].elapsed_ms.saturating_sub(pair[0].elapsed_ms);
        if elapsed == 0 {
            continue;
        }
        let Some(ticks) = opt_sum(
            delta(first.utime_ticks, last.utime_ticks),
            delta(first.stime_ticks, last.stime_ticks),
        ) else {
            continue;
        };
        let percent = ((ticks as f64 * 1000.0 / ASSUMED_CLK_TCK) / elapsed as f64) * 100.0;
        peak = Some(peak.map_or(percent, |current: f64| current.max(percent)));
    }
    peak
}

fn system_cpu_percent(samples: &[WorkloadSample]) -> Option<f64> {
    let first = samples.iter().find_map(|sample| sample.cpu_ticks)?;
    let last = samples.iter().rev().find_map(|sample| sample.cpu_ticks)?;
    let total_delta = last.0.checked_sub(first.0)?;
    let idle_delta = last.1.checked_sub(first.1)?;
    (total_delta > 0).then_some(((total_delta - idle_delta) as f64 / total_delta as f64) * 100.0)
}

fn opt_sum(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    Some(left? + right?)
}

fn delta(first: Option<u64>, last: Option<u64>) -> Option<u64> {
    last?.checked_sub(first?)
}

fn cpu_ticks() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/stat").ok()?;
    let line = text.lines().next()?;
    let values = line
        .split_whitespace()
        .skip(1)
        .filter_map(|part| part.parse::<u64>().ok())
        .collect::<Vec<_>>();
    let total = values.iter().sum();
    let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
    Some((total, idle))
}

fn mem_available_kb() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn avg_cpu_freq_khz() -> Option<u64> {
    let entries = fs::read_dir("/sys/devices/system/cpu/cpufreq").ok()?;
    let values = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("policy"))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            read_to_string_lossy(entry.path().join("scaling_cur_freq"))
                .ok()
                .flatten()?
                .parse::<u64>()
                .ok()
        })
        .collect::<Vec<_>>();
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<u64>() / values.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_validation_proc_stat_parser_handles_spaces_in_comm() {
        let text = "123 (name with spaces) S 1 2 3 4 5 6 7 8 9 10 130 140 15 16 17";
        let (utime, stime) = parse_proc_stat(text).unwrap();
        assert_eq!(utime, 130);
        assert_eq!(stime, 140);
    }
}
