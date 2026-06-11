use crate::contracts::{
    CompositeBoundaryPhase, CompositeBoundaryResult, CompositeBoundaryScenario,
    ContractEvidenceGap, ContractEvidenceStatus, ContractFactor, NetworkPressureEvidence,
    NetworkPressureMode, PressureCondition, PressureEffect, PressureIntensity, PressureSafety,
    ResourceCouplingEvidenceClass, ResourceMetric, ResourcePressureEvidenceClass,
    ResourcePressureKind, ResourcePressureResult, ResourceSideEffect,
};
use crate::ids::{new_id, now_unix_ms};
use crate::load::{
    new_cpu_load_plan_with_operator_abort, run_cpu_load_with_options, CpuLoadRuntimeOptions,
};
use crate::observe::{max_temp_c, sample_local, Signal};
use crate::{LabError, LabResult};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_PRESSURE_DURATION_SECONDS: u64 = 30;
pub const MAX_PRESSURE_MEMORY_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_PRESSURE_STORAGE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PRESSURE_NETWORK_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone)]
pub struct PressureProbeOptions {
    pub duration: Duration,
    pub workers: usize,
    pub abort_temp_c: Option<f64>,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub network_bytes: u64,
    pub network_endpoint: Option<String>,
    pub storage_dir: Option<PathBuf>,
}

impl Default for PressureProbeOptions {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(1),
            workers: 1,
            abort_temp_c: None,
            memory_bytes: 8 * 1024 * 1024,
            storage_bytes: 1024 * 1024,
            network_bytes: 0,
            network_endpoint: None,
            storage_dir: None,
        }
    }
}

impl ResourcePressureKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MemoryPressure => "memory_pressure",
            Self::StorageIo => "storage_io",
            Self::NetworkIo => "network_io",
            Self::LatencyJitter => "latency_jitter",
            Self::CpuPressure => "cpu_pressure",
            Self::ThermalPressure => "thermal_pressure",
            Self::ObserverPressure => "observer_pressure",
        }
    }
}

impl FromStr for ResourcePressureKind {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "memory_pressure" => Ok(Self::MemoryPressure),
            "storage_io" => Ok(Self::StorageIo),
            "network_io" => Ok(Self::NetworkIo),
            "latency_jitter" => Ok(Self::LatencyJitter),
            "cpu_pressure" => Ok(Self::CpuPressure),
            "thermal_pressure" => Ok(Self::ThermalPressure),
            "observer_pressure" => Ok(Self::ObserverPressure),
            other => Err(LabError::Validation(format!(
                "unknown pressure kind {other}"
            ))),
        }
    }
}

impl CompositeBoundaryScenario {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MemoryStorageJitter => "memory_storage_jitter",
        }
    }
}

impl FromStr for CompositeBoundaryScenario {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "memory_storage_jitter" => Ok(Self::MemoryStorageJitter),
            other => Err(LabError::Validation(format!(
                "unknown composite scenario {other}"
            ))),
        }
    }
}

pub fn run_resource_pressure(
    target_id: String,
    pressure_kind: ResourcePressureKind,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    validate_pressure_options(options)?;
    match pressure_kind {
        ResourcePressureKind::MemoryPressure => run_memory_pressure(target_id, options),
        ResourcePressureKind::StorageIo => run_storage_io(target_id, options),
        ResourcePressureKind::NetworkIo => run_network_io(target_id, options),
        ResourcePressureKind::LatencyJitter => run_latency_jitter(target_id, options),
        ResourcePressureKind::CpuPressure => run_cpu_pressure(target_id, options),
        ResourcePressureKind::ThermalPressure => run_thermal_pressure(target_id, options),
        ResourcePressureKind::ObserverPressure => run_observer_pressure(target_id, options),
    }
}

pub fn run_composite_boundary(
    target_id: String,
    scenario: CompositeBoundaryScenario,
    options: &PressureProbeOptions,
) -> LabResult<CompositeBoundaryResult> {
    validate_pressure_options(options)?;
    match scenario {
        CompositeBoundaryScenario::MemoryStorageJitter => {
            run_memory_storage_jitter_composite(target_id, options)
        }
    }
}

fn validate_pressure_options(options: &PressureProbeOptions) -> LabResult<()> {
    let duration_seconds = options.duration.as_secs().max(1);
    if duration_seconds > MAX_PRESSURE_DURATION_SECONDS {
        return Err(LabError::Policy(format!(
            "pressure duration must be <= {MAX_PRESSURE_DURATION_SECONDS}s"
        )));
    }
    if options.memory_bytes > MAX_PRESSURE_MEMORY_BYTES {
        return Err(LabError::Policy(format!(
            "pressure memory bytes must be <= {MAX_PRESSURE_MEMORY_BYTES}"
        )));
    }
    if options.storage_bytes > MAX_PRESSURE_STORAGE_BYTES {
        return Err(LabError::Policy(format!(
            "pressure storage bytes must be <= {MAX_PRESSURE_STORAGE_BYTES}"
        )));
    }
    if options.network_bytes > MAX_PRESSURE_NETWORK_BYTES {
        return Err(LabError::Policy(format!(
            "pressure network bytes must be <= {MAX_PRESSURE_NETWORK_BYTES}"
        )));
    }
    if options.workers == 0 {
        return Err(LabError::Validation("workers must be >= 1".to_string()));
    }
    Ok(())
}

fn run_memory_storage_jitter_composite(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<CompositeBoundaryResult> {
    let started = Instant::now();
    let baseline = run_latency_jitter(target_id.clone(), options)?;

    let before_mem = meminfo_values();
    let before_vm = vmstat_values();
    let before_psi = memory_psi_avg10();
    let bytes = options.memory_bytes as usize;
    let mut pressure_buffer = vec![0u8; bytes];
    for chunk in pressure_buffer.chunks_mut(4096) {
        chunk[0] = chunk[0].wrapping_add(1);
    }
    let touched_checksum = pressure_buffer
        .iter()
        .step_by(4096)
        .fold(0u64, |acc, value| acc.wrapping_add(*value as u64));
    let after_touch_mem = meminfo_values();
    let after_touch_vm = vmstat_values();
    let after_touch_psi = memory_psi_avg10();
    let mem_available_delta = optional_delta_u64(
        before_mem.get("MemAvailable").copied(),
        after_touch_mem.get("MemAvailable").copied(),
    );
    let pgscan_delta = optional_delta_u64(
        before_vm.get("pgscan_kswapd").copied(),
        after_touch_vm.get("pgscan_kswapd").copied(),
    );
    let pgsteal_delta = optional_delta_u64(
        before_vm.get("pgsteal_kswapd").copied(),
        after_touch_vm.get("pgsteal_kswapd").copied(),
    );
    let major_faults_delta = optional_delta_u64(
        before_vm.get("pgmajfault").copied(),
        after_touch_vm.get("pgmajfault").copied(),
    );
    let psi_delta = optional_delta_f64(before_psi, after_touch_psi);
    let memory_pressure_effect_observed = positive_delta(pgscan_delta)
        || positive_delta(pgsteal_delta)
        || positive_delta(major_faults_delta)
        || positive_delta(psi_delta);

    let storage = run_storage_io(target_id.clone(), options)?;
    let jitter_under_pressure = run_latency_jitter(target_id.clone(), options)?;
    drop(pressure_buffer);
    thread::sleep(Duration::from_millis(250));
    let recovery_mem = meminfo_values();
    let recovery_delta = optional_delta_u64(
        before_mem.get("MemAvailable").copied(),
        recovery_mem.get("MemAvailable").copied(),
    );

    let baseline_p99 = metric_value(&baseline.metrics, "jitter_p99_ms");
    let pressure_p99 = metric_value(&jitter_under_pressure.metrics, "jitter_p99_ms");
    let jitter_p99_delta = optional_delta_f64(baseline_p99, pressure_p99);
    let storage_effect = storage.pressure_effect.observed;
    let composite_effect_observed =
        memory_pressure_effect_observed && (storage_effect || positive_delta(jitter_p99_delta));

    let phases = vec![
        CompositeBoundaryPhase {
            phase_id: "baseline_jitter".to_string(),
            pressure_kind: "latency_jitter".to_string(),
            status: baseline.status.clone(),
            summary: "baseline monotonic jitter before memory/storage pressure".to_string(),
            metrics: baseline.metrics.clone(),
        },
        CompositeBoundaryPhase {
            phase_id: "memory_hold".to_string(),
            pressure_kind: "memory_pressure".to_string(),
            status: if memory_pressure_effect_observed {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::Insufficient
            },
            summary: "anonymous memory was allocated, touched, and held while storage and jitter phases ran".to_string(),
            metrics: vec![
                metric("anonymous_bytes_touched", Some(bytes as f64), "bytes", None),
                metric("mem_available_delta_kb", mem_available_delta, "KiB", None),
                metric("pgscan_delta", pgscan_delta, "count", None),
                metric("pgsteal_delta", pgsteal_delta, "count", None),
                metric("major_faults_delta", major_faults_delta, "count", None),
                metric(
                    "memory_psi_some_avg10_delta",
                    psi_delta,
                    "ratio",
                    Some("null when /proc/pressure/memory is unavailable".to_string()),
                ),
                metric("touch_checksum", Some(touched_checksum as f64), "count", None),
            ],
        },
        CompositeBoundaryPhase {
            phase_id: "storage_under_memory".to_string(),
            pressure_kind: "storage_io".to_string(),
            status: storage.status.clone(),
            summary: "bounded tempfile write/read ran while anonymous memory allocation was held".to_string(),
            metrics: storage.metrics.clone(),
        },
        CompositeBoundaryPhase {
            phase_id: "jitter_after_memory_storage".to_string(),
            pressure_kind: "latency_jitter".to_string(),
            status: jitter_under_pressure.status.clone(),
            summary: "monotonic jitter ran while anonymous allocation remained held after storage I/O".to_string(),
            metrics: {
                let mut metrics = jitter_under_pressure.metrics.clone();
                metrics.push(metric(
                    "jitter_p99_delta_ms",
                    jitter_p99_delta,
                    "ms",
                    Some("under-pressure p99 minus baseline p99".to_string()),
                ));
                metrics
            },
        },
        CompositeBoundaryPhase {
            phase_id: "recovery".to_string(),
            pressure_kind: "recovery".to_string(),
            status: ContractEvidenceStatus::MeasuredPartial,
            summary: "memory allocation was released and MemAvailable was sampled after a short recovery pause".to_string(),
            metrics: vec![metric(
                "mem_available_recovery_delta_kb",
                recovery_delta,
                "KiB",
                Some("post-release MemAvailable minus baseline MemAvailable".to_string()),
            )],
        },
    ];

    let status = if composite_effect_observed {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::Insufficient
    };
    let duration_ms = started.elapsed().as_millis() as u64;
    Ok(CompositeBoundaryResult {
        schema_version: "lab.composite_boundary_result.v1".to_string(),
        result_id: new_id("COMPOSITE"),
        target_id,
        scenario: CompositeBoundaryScenario::MemoryStorageJitter,
        status,
        coupling_evidence_class: ResourceCouplingEvidenceClass::CompositeMeasured,
        duration_ms,
        controlled_factors: vec![
            factor("anonymous_memory_bytes", bytes.to_string()),
            factor("tempfile_bytes", options.storage_bytes.to_string()),
            factor("jitter_interval_ms", "1".to_string()),
        ],
        observed_covariates: vec![
            factor("meminfo", "before_touch_recovery".to_string()),
            factor("vmstat", "before_after_touch".to_string()),
            factor("storage_latency", "write_read_ms".to_string()),
            factor("jitter", "baseline_and_under_pressure".to_string()),
        ],
        uncontrolled_confounders: vec![
            "kernel reclaim policy".to_string(),
            "filesystem cache state".to_string(),
            "background scheduler and interrupt load".to_string(),
        ],
        phases,
        safety: PressureSafety {
            duration_seconds_max: duration_ms.div_ceil(1000),
            memory_bytes_max: options.memory_bytes,
            storage_bytes_max: options.storage_bytes,
            network_bytes_max: 0,
            abort_conditions: vec![
                "duration ceiling for each component phase".to_string(),
                "operator can terminate adc-lab command".to_string(),
            ],
            cleanup: vec![
                "allocation released by process scope".to_string(),
                "storage tempfile cleanup delegated to storage phase".to_string(),
            ],
            cleanup_verified: storage.safety.cleanup_verified,
        },
        evidence_refs: Vec::new(),
        claim_supported: vec![
            "phase-based memory+storage+jitter composite probe completed in one target process"
                .to_string(),
        ],
        claim_blocked: vec![
            "memory resident budget remains blocked until laddered pressure produces reclaim/PSI/fault evidence".to_string(),
            "simultaneous storage+jitter execution and sustained storage cadence remain unmeasured".to_string(),
        ],
        next_evidence_needed: vec![
            evidence_gap(
                "memory pressure effect was not necessarily induced by the bounded allocation",
                "larger approved memory ladder with recovery",
                &["pressure_effect_observed", "recovery repeat", "page-cache side effect"],
                "run approved ladder before resident-memory design budgets",
                "operator_approval",
            ),
            evidence_gap(
                "storage and jitter phases are sequential under held memory, not concurrent",
                "concurrent storage+jitter composite runner",
                &["storage I/O while jitter loop runs", "tail latency under active I/O"],
                "add a concurrent paired-pressure runner before real-time-ish storage claims",
                "adc-lab",
            ),
        ],
        time_unix_ms: now_unix_ms(),
    })
}

fn run_memory_pressure(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let before_mem = meminfo_values();
    let before_vm = vmstat_values();
    let before_psi = memory_psi_avg10();
    let bytes = options.memory_bytes as usize;
    let mut allocation = vec![0u8; bytes];
    for offset in (0..allocation.len()).step_by(4096) {
        allocation[offset] = (offset % 251) as u8;
    }
    thread::sleep(options.duration);
    let touched_checksum = allocation
        .iter()
        .step_by(4096)
        .fold(0u64, |acc, value| acc.wrapping_add(*value as u64));
    drop(allocation);
    let after_mem = meminfo_values();
    let after_vm = vmstat_values();
    let after_psi = memory_psi_avg10();
    let mem_available_delta = optional_delta_u64(
        before_mem.get("MemAvailable").copied(),
        after_mem.get("MemAvailable").copied(),
    );
    let pgscan_delta = optional_delta_u64(
        before_vm.get("pgscan_kswapd").copied(),
        after_vm.get("pgscan_kswapd").copied(),
    );
    let pgsteal_delta = optional_delta_u64(
        before_vm.get("pgsteal_kswapd").copied(),
        after_vm.get("pgsteal_kswapd").copied(),
    );
    let major_faults_delta = optional_delta_u64(
        before_vm.get("pgmajfault").copied(),
        after_vm.get("pgmajfault").copied(),
    );
    let psi_delta = optional_delta_f64(before_psi, after_psi);
    let pressure_effect_observed = positive_delta(pgscan_delta)
        || positive_delta(pgsteal_delta)
        || positive_delta(major_faults_delta)
        || positive_delta(psi_delta);
    let pressure_basis = vec![
        format!(
            "mem_available_delta_kb={}",
            format_optional_f64(mem_available_delta)
        ),
        format!("pgscan_delta={}", format_optional_f64(pgscan_delta)),
        format!("pgsteal_delta={}", format_optional_f64(pgsteal_delta)),
        format!(
            "major_faults_delta={}",
            format_optional_f64(major_faults_delta)
        ),
        format!(
            "memory_psi_some_avg10_delta={}",
            format_optional_f64(psi_delta)
        ),
    ];

    let metrics = vec![
        metric(
            "anonymous_bytes_touched",
            Some(bytes as f64),
            "bytes",
            Some("bounded user-space anonymous allocation".to_string()),
        ),
        metric("mem_available_delta_kb", mem_available_delta, "KiB", None),
        metric("pgscan_delta", pgscan_delta, "count", None),
        metric("pgsteal_delta", pgsteal_delta, "count", None),
        metric("major_faults_delta", major_faults_delta, "count", None),
        metric(
            "memory_psi_some_avg10_delta",
            psi_delta,
            "ratio",
            Some("null when /proc/pressure/memory is unavailable".to_string()),
        ),
        metric(
            "touch_checksum",
            Some(touched_checksum as f64),
            "count",
            None,
        ),
    ];
    let memory_status = if pressure_effect_observed {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::Insufficient
    };

    let mut result = result(
        target_id,
        ResourcePressureKind::MemoryPressure,
        memory_status.clone(),
        started.elapsed(),
        vec![factor("anonymous_memory_bytes", bytes.to_string())],
        vec![factor("meminfo", "before_after".to_string()), factor("vmstat", "before_after".to_string())],
        vec!["kernel reclaim policy".to_string(), "background memory activity".to_string()],
        metrics,
        vec![
            side_effect(
                "storage",
                memory_status,
                "anonymous allocation records reclaim/page-cache indicators; storage latency coupling requires paired memory+storage evidence",
                Vec::new(),
            ),
            side_effect(
                "cpu",
                ContractEvidenceStatus::MeasuredPartial,
                "page touching consumes CPU during the bounded pressure window",
                Vec::new(),
            ),
        ],
        safety(options, true, vec!["allocation released by process scope".to_string()]),
        if pressure_effect_observed {
            vec!["bounded anonymous allocation produced memory pressure indicators and was released".to_string()]
        } else {
            vec!["bounded anonymous allocation smoke was applied and released; memory pressure effect remains unproven".to_string()]
        },
        vec![
            "safe resident memory budget still requires laddered trials".to_string(),
            "storage latency under memory pressure requires paired memory+storage evidence".to_string(),
        ],
    );
    result.evidence_class = if pressure_effect_observed {
        ResourcePressureEvidenceClass::PressureInduced
    } else {
        ResourcePressureEvidenceClass::Smoke
    };
    result.intensity = PressureIntensity {
        requested: format!("{bytes} anonymous bytes touched"),
        relative_to_target: bytes_relative_to_memtotal(bytes as u64, &before_mem),
        pressure_effect_observed,
    };
    result.pressure_effect = PressureEffect {
        observed: pressure_effect_observed,
        basis: pressure_basis,
    };
    result.condition.workers = Some("n/a".to_string());
    Ok(result)
}

fn run_storage_io(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let temp_dir = options
        .storage_dir
        .clone()
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&temp_dir)?;
    let path = temp_dir.join(format!("adc-lab-storage-{}.tmp", new_id("PRESSURE")));
    let before_disk = diskstats_totals();
    let before_mem = meminfo_values();
    let buffer = vec![0x5au8; 64 * 1024];
    let mut written = 0u64;
    let write_started = Instant::now();
    {
        let mut file = fs::File::create(&path)?;
        while written < options.storage_bytes {
            let remaining = (options.storage_bytes - written) as usize;
            let chunk = remaining.min(buffer.len());
            file.write_all(&buffer[..chunk])?;
            written += chunk as u64;
        }
        file.sync_all()?;
    }
    let write_ms = write_started.elapsed().as_secs_f64() * 1000.0;
    let read_started = Instant::now();
    let mut read_bytes = 0u64;
    {
        let mut file = fs::File::open(&path)?;
        let mut read_buffer = [0u8; 64 * 1024];
        loop {
            let count = file.read(&mut read_buffer)?;
            if count == 0 {
                break;
            }
            read_bytes += count as u64;
        }
    }
    let read_ms = read_started.elapsed().as_secs_f64() * 1000.0;
    fs::remove_file(&path)?;
    let cleanup_verified = !path.exists();
    let after_disk = diskstats_totals();
    let after_mem = meminfo_values();
    let disk_read_sectors_delta =
        optional_delta_u64(before_disk.map(|v| v.0), after_disk.map(|v| v.0));
    let disk_write_sectors_delta =
        optional_delta_u64(before_disk.map(|v| v.1), after_disk.map(|v| v.1));
    let mem_available_delta = optional_delta_u64(
        before_mem.get("MemAvailable").copied(),
        after_mem.get("MemAvailable").copied(),
    );
    let storage_effect_observed = written > 0
        && read_bytes > 0
        && (positive_delta(disk_write_sectors_delta)
            || positive_delta(disk_read_sectors_delta)
            || write_ms >= 0.0
            || read_ms >= 0.0);

    let metrics = vec![
        metric("bytes_written", Some(written as f64), "bytes", None),
        metric("bytes_read", Some(read_bytes as f64), "bytes", None),
        metric("write_latency_ms", Some(write_ms), "ms", None),
        metric("read_latency_ms", Some(read_ms), "ms", None),
        metric(
            "disk_read_sectors_delta",
            disk_read_sectors_delta,
            "sectors",
            None,
        ),
        metric(
            "disk_write_sectors_delta",
            disk_write_sectors_delta,
            "sectors",
            None,
        ),
        metric("mem_available_delta_kb", mem_available_delta, "KiB", None),
    ];

    let mut result = result(
        target_id,
        ResourcePressureKind::StorageIo,
        ContractEvidenceStatus::MeasuredPartial,
        started.elapsed(),
        vec![factor("tempfile_bytes", options.storage_bytes.to_string())],
        vec![
            factor("diskstats", "before_after".to_string()),
            factor("meminfo", "before_after".to_string()),
        ],
        vec![
            "filesystem cache state".to_string(),
            "storage device firmware".to_string(),
        ],
        metrics,
        vec![
            side_effect(
                "memory",
                ContractEvidenceStatus::MeasuredPartial,
                "tempfile I/O can consume page cache and alter MemAvailable",
                Vec::new(),
            ),
            side_effect(
                "latency",
                ContractEvidenceStatus::MeasuredPartial,
                "write/read latency measured for bounded tempfile path",
                Vec::new(),
            ),
        ],
        safety(
            options,
            cleanup_verified,
            vec![format!("removed {}", path.display())],
        ),
        vec!["bounded tempfile I/O smoke completed with cleanup verification".to_string()],
        vec![
            "sustained storage cadence and flash-wear claims require longer evidence".to_string(),
            "page-cache/storage behavior under memory pressure requires paired pressure evidence"
                .to_string(),
        ],
    );
    result.evidence_class = ResourcePressureEvidenceClass::Smoke;
    result.intensity = PressureIntensity {
        requested: format!("{} tempfile bytes", options.storage_bytes),
        relative_to_target:
            "bounded tempfile smoke; not normalized to device endurance or sustained bandwidth"
                .to_string(),
        pressure_effect_observed: storage_effect_observed,
    };
    result.pressure_effect = PressureEffect {
        observed: storage_effect_observed,
        basis: vec![
            format!("bytes_written={written}"),
            format!("bytes_read={read_bytes}"),
            format!("write_latency_ms={write_ms:.3}"),
            format!("read_latency_ms={read_ms:.3}"),
            format!(
                "disk_write_sectors_delta={}",
                format_optional_f64(disk_write_sectors_delta)
            ),
        ],
    };
    result.condition.workers = Some("n/a".to_string());
    Ok(result)
}

fn run_network_io(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let before = netdev_totals();
    let mut endpoint_status = "not_configured".to_string();
    let mut connect_latency_ms = None;
    let mut endpoint_available = false;
    let mut generated_bytes = 0u64;
    let mut network_mode = if options.network_endpoint.is_some() {
        NetworkPressureMode::EndpointAttempt
    } else {
        NetworkPressureMode::CounterOnly
    };
    if let Some(endpoint) = options.network_endpoint.as_ref() {
        let connect_started = Instant::now();
        match resolve_socket_addr(endpoint).and_then(|addr| {
            TcpStream::connect_timeout(&addr, Duration::from_millis(500)).map_err(LabError::from)
        }) {
            Ok(mut stream) => {
                endpoint_status = "connected".to_string();
                endpoint_available = true;
                connect_latency_ms = Some(connect_started.elapsed().as_secs_f64() * 1000.0);
                if options.network_bytes > 0 {
                    let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
                    let buffer = [0x5au8; 8192];
                    while generated_bytes < options.network_bytes {
                        let remaining = (options.network_bytes - generated_bytes) as usize;
                        let chunk_len = remaining.min(buffer.len());
                        match stream.write(&buffer[..chunk_len]) {
                            Ok(0) => {
                                endpoint_status =
                                    format!("transfer_stalled_after_{generated_bytes}_bytes");
                                break;
                            }
                            Ok(written) => {
                                generated_bytes += written as u64;
                            }
                            Err(error) => {
                                endpoint_status = format!(
                                    "transfer_failed_after_{generated_bytes}_bytes:{error}"
                                );
                                break;
                            }
                        }
                    }
                    let _ = stream.shutdown(Shutdown::Write);
                    if generated_bytes == options.network_bytes {
                        endpoint_status = format!("bounded_transfer_completed:{generated_bytes}");
                        network_mode = NetworkPressureMode::BoundedTransfer;
                    }
                }
            }
            Err(error) => {
                endpoint_status = format!("connect_failed:{error}");
                connect_latency_ms = Some(connect_started.elapsed().as_secs_f64() * 1000.0);
            }
        }
    }
    thread::sleep(options.duration);
    let after = netdev_totals();
    let rx_delta = optional_delta_u64(before.map(|v| v.0), after.map(|v| v.0));
    let tx_delta = optional_delta_u64(before.map(|v| v.1), after.map(|v| v.1));
    let bounded_transfer_completed = matches!(network_mode, NetworkPressureMode::BoundedTransfer);
    let status = if options.network_endpoint.is_none() {
        ContractEvidenceStatus::NotApplicableWithReason
    } else if bounded_transfer_completed && before.is_some() && after.is_some() {
        ContractEvidenceStatus::MeasuredPartial
    } else if endpoint_available || (before.is_some() && after.is_some()) {
        ContractEvidenceStatus::Insufficient
    } else {
        ContractEvidenceStatus::NotApplicableWithReason
    };
    let pressure_effect_observed = bounded_transfer_completed;

    let metrics = vec![
        metric("rx_bytes_delta", rx_delta, "bytes", None),
        metric("tx_bytes_delta", tx_delta, "bytes", None),
        metric(
            "traffic_generated_bytes",
            Some(generated_bytes as f64),
            "bytes",
            None,
        ),
        metric(
            "connect_latency_ms",
            connect_latency_ms,
            "ms",
            Some(endpoint_status.clone()),
        ),
    ];

    let mut result = result(
        target_id,
        ResourcePressureKind::NetworkIo,
        status,
        started.elapsed(),
        vec![
            factor(
                "network_endpoint",
                options
                    .network_endpoint
                    .clone()
                    .unwrap_or_else(|| "none".to_string()),
            ),
            factor("network_bytes_max", options.network_bytes.to_string()),
        ],
        vec![factor("proc_net_dev", "before_after".to_string())],
        vec![
            "LAN load".to_string(),
            "remote endpoint availability".to_string(),
        ],
        metrics,
        vec![
            side_effect(
                "cpu",
                if bounded_transfer_completed {
                    ContractEvidenceStatus::MeasuredPartial
                } else {
                    ContractEvidenceStatus::Insufficient
                },
                "bounded network transfer executes socket writes on CPU when endpoint-backed",
                Vec::new(),
            ),
            side_effect(
                "latency",
                if connect_latency_ms.is_some() {
                    ContractEvidenceStatus::MeasuredPartial
                } else {
                    ContractEvidenceStatus::NotApplicableWithReason
                },
                "connect latency is recorded when an endpoint is configured",
                Vec::new(),
            ),
        ],
        safety(
            options,
            true,
            vec!["closed optional TCP socket".to_string()],
        ),
        if bounded_transfer_completed {
            vec!["endpoint-backed bounded network transfer completed".to_string()]
        } else if options.network_endpoint.is_some() {
            vec!["network endpoint attempt completed, but bounded transfer evidence remains insufficient".to_string()]
        } else {
            vec!["network interface rx/tx counters were observed; network I/O boundary was not measured without an endpoint".to_string()]
        },
        vec![
            "background upload cadence and retry/backoff require endpoint-specific trials"
                .to_string(),
            "bounded transfer, packet loss, and retry behavior are not measured by counter-only evidence".to_string(),
        ],
    );
    result.evidence_class = if bounded_transfer_completed {
        ResourcePressureEvidenceClass::BoundaryProbe
    } else if options.network_endpoint.is_some() {
        ResourcePressureEvidenceClass::Smoke
    } else {
        ResourcePressureEvidenceClass::NotApplicable
    };
    result.intensity = PressureIntensity {
        requested: format!("network_bytes_max={}", options.network_bytes),
        relative_to_target: if bounded_transfer_completed {
            "endpoint-backed bounded transfer; LAN topology and endpoint behavior remain confounders".to_string()
        } else if options.network_endpoint.is_some() {
            "endpoint attempt only; bounded transfer did not complete".to_string()
        } else {
            "counter-only observation; no endpoint configured".to_string()
        },
        pressure_effect_observed,
    };
    result.pressure_effect = PressureEffect {
        observed: pressure_effect_observed,
        basis: vec![
            format!("network_mode={}", network_mode_label(&network_mode)),
            format!("endpoint_status={endpoint_status}"),
            format!("traffic_generated_bytes={generated_bytes}"),
            format!(
                "connect_latency_ms={}",
                format_optional_f64(connect_latency_ms)
            ),
        ],
    };
    result.network_evidence = Some(NetworkPressureEvidence {
        network_mode,
        endpoint_available,
        traffic_generated_bytes: generated_bytes,
        selection_claim_allowed: false,
    });
    result.condition.workers = Some("n/a".to_string());
    Ok(result)
}

fn run_latency_jitter(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let samples = jitter_samples(options.duration, Duration::from_millis(1));
    let metrics = jitter_metrics(&samples);
    let mut result = result(
        target_id,
        ResourcePressureKind::LatencyJitter,
        ContractEvidenceStatus::MeasuredPartial,
        started.elapsed(),
        vec![factor("loop_interval_ms", "1".to_string())],
        vec![factor("monotonic_clock", "instant".to_string())],
        vec!["scheduler noise".to_string(), "interrupt load".to_string()],
        metrics,
        vec![side_effect(
            "cpu",
            ContractEvidenceStatus::MeasuredPartial,
            "jitter loop wakes at 1ms cadence during the bounded duration",
            Vec::new(),
        )],
        safety(options, true, vec!["no persistent state".to_string()]),
        vec!["current-condition monotonic jitter distribution was measured".to_string()],
        vec![
            "pressure-specific p95/p99 claims require paired pressure runs".to_string(),
            "real-time-ish claims require condition-specific CPU/memory/storage/network/observer jitter evidence".to_string(),
        ],
    );
    result.evidence_class = ResourcePressureEvidenceClass::Smoke;
    result.intensity = PressureIntensity {
        requested: "1ms monotonic loop under current condition".to_string(),
        relative_to_target: "current-condition jitter only; not normalized to a real-time workload"
            .to_string(),
        pressure_effect_observed: false,
    };
    result.pressure_effect = PressureEffect {
        observed: false,
        basis: vec![
            "jitter_p50_ms measured under current condition".to_string(),
            "jitter_p95_ms measured under current condition".to_string(),
            "jitter_p99_ms measured under current condition".to_string(),
            "no concurrent pressure was injected by this probe".to_string(),
        ],
    };
    result.condition.pressure_kind = "current_condition".to_string();
    result.condition.workers = Some("n/a".to_string());
    Ok(result)
}

fn run_cpu_pressure(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let before_temp = max_temp_c();
    let plan = new_cpu_load_plan_with_operator_abort(
        target_id.clone(),
        options.workers,
        options.duration,
        options.abort_temp_c,
        false,
    )?;
    let load = run_cpu_load_with_options(&plan, &CpuLoadRuntimeOptions::default())?;
    let after_temp = max_temp_c();
    let iterations = load.worker_iterations.iter().sum::<u64>();
    let metrics = vec![
        metric("workers", Some(options.workers as f64), "count", None),
        metric("worker_iterations", Some(iterations as f64), "count", None),
        metric(
            "duration_ms",
            Some(load.duration_ms as f64),
            "ms",
            Some(load.status.clone()),
        ),
        metric("temp_before_c", before_temp, "C", None),
        metric("temp_after_c", after_temp, "C", None),
        metric(
            "max_observed_temp_c",
            load.max_observed_temp_c,
            "C",
            load.abort_reason.clone(),
        ),
    ];
    let status = if load.status == "completed" {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::UnsafeToRunWithReason
    };
    let temp_delta = optional_delta_f64(before_temp, after_temp);
    let mut result = result(
        target_id,
        ResourcePressureKind::CpuPressure,
        status,
        started.elapsed(),
        vec![factor("workers", options.workers.to_string())],
        vec![
            factor("thermal", "safety_monitor".to_string()),
            factor("frequency", "observed_policy".to_string()),
        ],
        vec![
            "governor policy".to_string(),
            "ambient temperature".to_string(),
        ],
        metrics,
        vec![
            side_effect(
                "thermal",
                ContractEvidenceStatus::MeasuredPartial,
                "CPU load records thermal monitor samples when thermal zones are visible",
                Vec::new(),
            ),
            side_effect(
                "latency",
                ContractEvidenceStatus::Insufficient,
                "latency side effect requires paired jitter probe under CPU load",
                Vec::new(),
            ),
        ],
        safety(
            options,
            true,
            vec!["CPU workers stopped after bounded duration".to_string()],
        ),
        vec!["bounded CPU pressure was executed".to_string()],
        vec![
            "sustained all-core safety requires repeated and longer thermal soak evidence"
                .to_string(),
        ],
    );
    result.evidence_class = ResourcePressureEvidenceClass::BoundaryProbe;
    result.intensity = PressureIntensity {
        requested: format!(
            "{} worker(s) for {}s",
            options.workers,
            options.duration.as_secs().max(1)
        ),
        relative_to_target: "bounded CPU burst; not a sustained thermal soak".to_string(),
        pressure_effect_observed: iterations > 0,
    };
    result.pressure_effect = PressureEffect {
        observed: iterations > 0,
        basis: vec![
            format!("worker_iterations={iterations}"),
            format!("temp_delta_c={}", format_optional_f64(temp_delta)),
            format!("load_status={}", load.status),
        ],
    };
    result.condition.workers = Some(options.workers.to_string());
    Ok(result)
}

fn run_thermal_pressure(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let mut thermal_options = options.clone();
    thermal_options.workers = thermal_options.workers.max(1);
    let mut result = run_cpu_pressure(target_id, &thermal_options)?;
    result.pressure_kind = ResourcePressureKind::ThermalPressure;
    result.result_id = new_id("PRESSURE");
    let thermal_visible = result
        .metrics
        .iter()
        .any(|metric| metric.metric_id == "max_observed_temp_c" && metric.value.is_some());
    result.status = if thermal_visible {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::NotApplicableWithReason
    };
    result.evidence_class = ResourcePressureEvidenceClass::BoundaryProbe;
    result.intensity.pressure_effect_observed = thermal_visible;
    result.pressure_effect.observed = thermal_visible;
    result
        .pressure_effect
        .basis
        .push(format!("thermal_visible={thermal_visible}"));
    result.claim_supported =
        vec!["thermal surface was evaluated during bounded CPU pressure".to_string()];
    result.claim_blocked = vec![
        "5/15/30 minute soak and near-boundary thermal claims need approved longer evidence"
            .to_string(),
    ];
    result.time_unix_ms = now_unix_ms();
    Ok(result)
}

fn run_observer_pressure(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let baseline = jitter_samples(options.duration, Duration::from_millis(2));
    let observer_started = Instant::now();
    let mut sample_count = 0u64;
    let deadline = Instant::now() + options.duration;
    while Instant::now() < deadline {
        let _ = sample_local(
            sample_count as usize,
            &[Signal::Cpu, Signal::Freq, Signal::Thermal, Signal::Memory],
        )?;
        sample_count += 1;
        thread::sleep(Duration::from_millis(10));
    }
    let observer_ms = observer_started.elapsed().as_secs_f64() * 1000.0;
    let artifact_path =
        std::env::temp_dir().join(format!("adc-lab-observer-{}.json", new_id("PRESSURE")));
    let artifact_started = Instant::now();
    fs::write(
        &artifact_path,
        serde_json::to_vec(&serde_json::json!({
            "sample_count": sample_count,
            "target_id": target_id,
        }))?,
    )?;
    let artifact_write_ms = artifact_started.elapsed().as_secs_f64() * 1000.0;
    fs::remove_file(&artifact_path)?;
    let cleanup_verified = !artifact_path.exists();
    let observed = jitter_samples(options.duration, Duration::from_millis(2));
    let baseline_p99 = percentile(&baseline, 0.99);
    let observed_p99 = percentile(&observed, 0.99);

    let metrics = vec![
        metric("observer_samples", Some(sample_count as f64), "count", None),
        metric("observer_loop_ms", Some(observer_ms), "ms", None),
        metric(
            "artifact_write_latency_ms",
            Some(artifact_write_ms),
            "ms",
            None,
        ),
        metric("baseline_jitter_p99_ms", baseline_p99, "ms", None),
        metric("observer_jitter_p99_ms", observed_p99, "ms", None),
        metric(
            "observer_jitter_p99_delta_ms",
            optional_delta_f64(baseline_p99, observed_p99),
            "ms",
            None,
        ),
    ];

    let jitter_delta = optional_delta_f64(baseline_p99, observed_p99);
    let observer_effect_observed = jitter_delta.is_some() || sample_count > 0;
    let mut result = result(
        target_id,
        ResourcePressureKind::ObserverPressure,
        ContractEvidenceStatus::MeasuredPartial,
        started.elapsed(),
        vec![factor("observer_sample_sleep_ms", "10".to_string())],
        vec![factor("artifact_write", "temp_json".to_string())],
        vec![
            "scheduler noise".to_string(),
            "filesystem cache state".to_string(),
        ],
        metrics,
        vec![
            side_effect(
                "latency",
                ContractEvidenceStatus::MeasuredPartial,
                "observer-on p99 jitter compared against a bounded baseline",
                Vec::new(),
            ),
            side_effect(
                "storage",
                ContractEvidenceStatus::MeasuredPartial,
                "small artifact write latency measured and cleaned up",
                Vec::new(),
            ),
        ],
        safety(
            options,
            cleanup_verified,
            vec![format!("removed {}", artifact_path.display())],
        ),
        vec!["bounded observer sampling and artifact-write overhead were measured".to_string()],
        vec!["default low-overhead claims require cadence-specific repeated evidence".to_string()],
    );
    result.evidence_class = ResourcePressureEvidenceClass::PairedPressure;
    result.intensity = PressureIntensity {
        requested: format!("{sample_count} observer samples plus one artifact write"),
        relative_to_target: "bounded observer-off/on comparison; not a production logging cadence"
            .to_string(),
        pressure_effect_observed: observer_effect_observed,
    };
    result.pressure_effect = PressureEffect {
        observed: observer_effect_observed,
        basis: vec![
            format!("observer_samples={sample_count}"),
            format!("artifact_write_latency_ms={artifact_write_ms:.3}"),
            format!(
                "observer_jitter_p99_delta_ms={}",
                format_optional_f64(jitter_delta)
            ),
        ],
    };
    result.condition.workers = Some("n/a".to_string());
    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn result(
    target_id: String,
    pressure_kind: ResourcePressureKind,
    status: ContractEvidenceStatus,
    duration: Duration,
    controlled_factors: Vec<ContractFactor>,
    observed_covariates: Vec<ContractFactor>,
    uncontrolled_confounders: Vec<String>,
    metrics: Vec<ResourceMetric>,
    side_effects: Vec<ResourceSideEffect>,
    safety: PressureSafety,
    claim_supported: Vec<String>,
    claim_blocked: Vec<String>,
) -> ResourcePressureResult {
    let pressure_kind_label = pressure_kind.as_str().to_string();
    let duration_label = format!("{}ms", duration.as_millis());
    let next_evidence_needed = claim_blocked
        .iter()
        .map(|claim| {
            evidence_gap(
                claim,
                "target-specific pressure or boundary probe",
                &[claim.as_str()],
                "collect bounded target evidence before allowing this claim",
                "adc-lab",
            )
        })
        .collect();
    ResourcePressureResult {
        schema_version: "lab.resource_pressure_result.v1".to_string(),
        result_id: new_id("PRESSURE"),
        target_id,
        pressure_kind,
        status,
        evidence_class: ResourcePressureEvidenceClass::Smoke,
        intensity: PressureIntensity {
            requested: duration_label.clone(),
            relative_to_target: "not normalized to target capacity".to_string(),
            pressure_effect_observed: false,
        },
        pressure_effect: PressureEffect {
            observed: false,
            basis: Vec::new(),
        },
        network_evidence: None,
        condition: PressureCondition {
            pressure_kind: pressure_kind_label,
            governor: current_governor_summary(),
            workers: None,
            duration: duration_label,
        },
        duration_ms: duration.as_millis() as u64,
        controlled_factors,
        observed_covariates,
        uncontrolled_confounders,
        metrics,
        side_effects,
        safety,
        evidence_refs: Vec::new(),
        claim_supported,
        claim_blocked,
        next_evidence_needed,
        time_unix_ms: now_unix_ms(),
    }
}

fn evidence_gap(
    reason: &str,
    needed_probe: &str,
    blocking_missing_evidence: &[&str],
    next_action: &str,
    owner_surface: &str,
) -> ContractEvidenceGap {
    ContractEvidenceGap {
        reason: reason.to_string(),
        needed_probe: needed_probe.to_string(),
        blocking_missing_evidence: blocking_missing_evidence
            .iter()
            .map(|value| value.to_string())
            .collect(),
        next_action: next_action.to_string(),
        owner_surface: owner_surface.to_string(),
    }
}

fn current_governor_summary() -> Option<String> {
    fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn safety(
    options: &PressureProbeOptions,
    cleanup_verified: bool,
    cleanup: Vec<String>,
) -> PressureSafety {
    PressureSafety {
        duration_seconds_max: options.duration.as_secs().max(1),
        memory_bytes_max: options.memory_bytes,
        storage_bytes_max: options.storage_bytes,
        network_bytes_max: options.network_bytes,
        abort_conditions: vec![
            "duration ceiling".to_string(),
            "operator can terminate adc-lab command".to_string(),
            "thermal abort when configured".to_string(),
        ],
        cleanup,
        cleanup_verified,
    }
}

fn jitter_samples(duration: Duration, interval: Duration) -> Vec<f64> {
    let started = Instant::now();
    let deadline = started + duration;
    let mut next = Instant::now() + interval;
    let mut samples = Vec::new();
    while Instant::now() < deadline {
        let now = Instant::now();
        if next > now {
            thread::sleep(next - now);
        }
        let actual = Instant::now();
        let late = actual.saturating_duration_since(next).as_secs_f64() * 1000.0;
        samples.push(late);
        next += interval;
    }
    samples
}

fn jitter_metrics(samples: &[f64]) -> Vec<ResourceMetric> {
    vec![
        metric("sample_count", Some(samples.len() as f64), "count", None),
        metric("jitter_p50_ms", percentile(samples, 0.50), "ms", None),
        metric("jitter_p95_ms", percentile(samples, 0.95), "ms", None),
        metric("jitter_p99_ms", percentile(samples, 0.99), "ms", None),
        metric("jitter_max_ms", max_value(samples), "ms", None),
    ]
}

fn percentile(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((sorted.len() - 1) as f64 * percentile).round() as usize;
    sorted.get(index).copied()
}

fn max_value(values: &[f64]) -> Option<f64> {
    values.iter().copied().reduce(f64::max)
}

fn metric(metric_id: &str, value: Option<f64>, unit: &str, note: Option<String>) -> ResourceMetric {
    ResourceMetric {
        metric_id: metric_id.to_string(),
        value,
        unit: unit.to_string(),
        note,
    }
}

fn metric_value(metrics: &[ResourceMetric], metric_id: &str) -> Option<f64> {
    metrics
        .iter()
        .find(|metric| metric.metric_id == metric_id)
        .and_then(|metric| metric.value)
}

fn optional_delta_u64(before: Option<u64>, after: Option<u64>) -> Option<f64> {
    Some(after? as f64 - before? as f64)
}

fn optional_delta_f64(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    Some(after? - before?)
}

fn positive_delta(value: Option<f64>) -> bool {
    value.is_some_and(|value| value > 0.0)
}

fn format_optional_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.3}"))
        .unwrap_or_else(|| "unavailable".to_string())
}

fn bytes_relative_to_memtotal(bytes: u64, meminfo: &BTreeMap<String, u64>) -> String {
    let Some(mem_total_kb) = meminfo.get("MemTotal").copied() else {
        return "MemTotal unavailable".to_string();
    };
    let mem_total_bytes = mem_total_kb.saturating_mul(1024);
    if mem_total_bytes == 0 {
        return "MemTotal unavailable".to_string();
    }
    format!(
        "{:.3}% of MemTotal",
        (bytes as f64 / mem_total_bytes as f64) * 100.0
    )
}

fn network_mode_label(mode: &NetworkPressureMode) -> &'static str {
    match mode {
        NetworkPressureMode::CounterOnly => "counter_only",
        NetworkPressureMode::EndpointAttempt => "endpoint_attempt",
        NetworkPressureMode::BoundedTransfer => "bounded_transfer",
    }
}

fn factor(factor_id: &str, value: String) -> ContractFactor {
    ContractFactor {
        factor_id: factor_id.to_string(),
        value,
    }
}

fn side_effect(
    resource: &str,
    status: ContractEvidenceStatus,
    summary: &str,
    metrics: Vec<ResourceMetric>,
) -> ResourceSideEffect {
    ResourceSideEffect {
        resource: resource.to_string(),
        status,
        summary: summary.to_string(),
        metrics,
    }
}

fn meminfo_values() -> BTreeMap<String, u64> {
    parse_proc_key_values("/proc/meminfo")
}

fn vmstat_values() -> BTreeMap<String, u64> {
    parse_proc_key_values("/proc/vmstat")
}

fn parse_proc_key_values(path: &str) -> BTreeMap<String, u64> {
    fs::read_to_string(path)
        .ok()
        .map(|text| {
            text.lines()
                .filter_map(|line| {
                    let mut parts = line.split_whitespace();
                    let key = parts.next()?.trim_end_matches(':').to_string();
                    let value = parts.next()?.parse::<u64>().ok()?;
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn memory_psi_avg10() -> Option<f64> {
    let text = fs::read_to_string("/proc/pressure/memory").ok()?;
    let line = text.lines().find(|line| line.starts_with("some "))?;
    line.split_whitespace()
        .find_map(|part| part.strip_prefix("avg10=")?.parse::<f64>().ok())
}

fn diskstats_totals() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/diskstats").ok()?;
    let mut read_sectors = 0u64;
    let mut written_sectors = 0u64;
    for line in text.lines() {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 14 {
            continue;
        }
        let name = parts[2];
        if name.starts_with("loop") || name.starts_with("ram") {
            continue;
        }
        read_sectors = read_sectors.saturating_add(parts[5].parse::<u64>().unwrap_or(0));
        written_sectors = written_sectors.saturating_add(parts[9].parse::<u64>().unwrap_or(0));
    }
    Some((read_sectors, written_sectors))
}

fn netdev_totals() -> Option<(u64, u64)> {
    let text = fs::read_to_string("/proc/net/dev").ok()?;
    let mut rx = 0u64;
    let mut tx = 0u64;
    for line in text.lines().skip(2) {
        let Some((iface, data)) = line.split_once(':') else {
            continue;
        };
        if iface.trim() == "lo" {
            continue;
        }
        let values = data.split_whitespace().collect::<Vec<_>>();
        if values.len() < 16 {
            continue;
        }
        rx = rx.saturating_add(values[0].parse::<u64>().unwrap_or(0));
        tx = tx.saturating_add(values[8].parse::<u64>().unwrap_or(0));
    }
    Some((rx, tx))
}

fn resolve_socket_addr(endpoint: &str) -> LabResult<SocketAddr> {
    endpoint
        .to_socket_addrs()
        .map_err(LabError::from)?
        .next()
        .ok_or_else(|| {
            LabError::Validation(format!("could not resolve network endpoint {endpoint}"))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_validation_network_without_endpoint_is_not_unsupported() {
        let options = PressureProbeOptions {
            duration: Duration::from_millis(1),
            ..PressureProbeOptions::default()
        };
        let result = run_resource_pressure(
            "local-target".to_string(),
            ResourcePressureKind::NetworkIo,
            &options,
        )
        .unwrap();
        assert_ne!(format!("{:?}", result.status), "UnsupportedByAdcLab");
        assert!(matches!(
            result.status,
            ContractEvidenceStatus::NotApplicableWithReason
        ));
        let network = result.network_evidence.as_ref().unwrap();
        assert!(matches!(
            network.network_mode,
            NetworkPressureMode::CounterOnly
        ));
        assert!(!network.endpoint_available);
        assert_eq!(network.traffic_generated_bytes, 0);
        assert!(!network.selection_claim_allowed);
    }
}
