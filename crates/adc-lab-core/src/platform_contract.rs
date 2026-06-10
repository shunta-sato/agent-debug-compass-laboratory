use crate::contracts::{
    BoundaryProbe, BoundaryProbePlan, ContractConfidence, ContractEvidenceStatus, ContractFactor,
    OperatingBoundary, OperatingContractRule, OperatingRuleCategory, PlatformMechanism,
    PlatformMechanismInventory, PressureSafety, ResourceCouplingChain, ResourceCouplingReport,
    ResourceMetric, ResourcePressureKind, ResourcePressureResult, ResourceSideEffect,
    TargetInventory, TargetOperatingContract, TargetOperatingContractStatus,
};
use crate::ids::{new_id, now_unix_ms};
use crate::load::{
    new_cpu_load_plan_with_operator_abort, run_cpu_load_with_options, CpuLoadRuntimeOptions,
};
use crate::observe::{max_temp_c, sample_local, Signal};
use crate::run::{artifact_uri_for_run, run_id_from_run_dir};
use crate::{LabError, LabResult};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
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

pub fn platform_mechanism_inventory_for_run(
    run_dir: impl AsRef<Path>,
    target_id: String,
    target_class: String,
) -> LabResult<PlatformMechanismInventory> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_run_dir(run_dir);
    let inventory_path = run_dir.join("inventory/target_inventory.json");
    let inventory = read_json_if_exists::<TargetInventory>(&inventory_path)?;
    let inventory_ref =
        artifact_ref_if_exists(run_dir, &run_id, "inventory/target_inventory.json")?;
    let pressure_refs = pressure_result_refs(run_dir, &run_id)?;
    let pressure_kinds = pressure_kind_set(run_dir)?;
    let has_pressure = |kind: ResourcePressureKind| pressure_kinds.contains(kind.as_str());

    let mut evidence_refs = Vec::new();
    if let Some(reference) = inventory_ref.clone() {
        evidence_refs.push(reference);
    }
    evidence_refs.extend(pressure_refs.values().cloned());

    let cpufreq_policies = inventory
        .as_ref()
        .map(|value| value.hardware.cpufreq_policies)
        .unwrap_or(0);
    let thermal_zones = inventory
        .as_ref()
        .map(|value| value.hardware.thermal_zones)
        .unwrap_or(0);
    let memory_total = inventory
        .as_ref()
        .and_then(|value| value.hardware.memory_total_kb);
    let cpu_count = inventory
        .as_ref()
        .map(|value| value.hardware.cpu_count)
        .unwrap_or(0);
    let cpufreq_surface = inventory
        .as_ref()
        .is_some_and(|value| surface_available(value, "linux.cpufreq.sysfs"));

    let mechanisms = vec![
        PlatformMechanism {
            domain: "compute".to_string(),
            mechanism_id: "cpu.core_count".to_string(),
            description: format!("online CPU core count observed as {cpu_count}"),
            visibility_status: if inventory.is_some() {
                ContractEvidenceStatus::Measured
            } else {
                ContractEvidenceStatus::Insufficient
            },
            control_status: ContractEvidenceStatus::NotControllable,
            evidence_refs: inventory_ref.clone().into_iter().collect(),
            reason: "core count is visible through target inventory; core online/offline control is not exposed by adc-lab pressure probes".to_string(),
        },
        PlatformMechanism {
            domain: "compute".to_string(),
            mechanism_id: "cpu.cpufreq_governor".to_string(),
            description: "CPU frequency policy and governor surface".to_string(),
            visibility_status: if cpufreq_policies > 0 {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::NotApplicableWithReason
            },
            control_status: if cpufreq_surface {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::NotControllable
            },
            evidence_refs: inventory_ref.clone().into_iter().collect(),
            reason: if cpufreq_surface {
                "cpufreq sysfs surface is visible; privileged restore-safe control remains a separate approved operation".to_string()
            } else {
                "cpufreq sysfs control surface was not visible in target inventory".to_string()
            },
        },
        PlatformMechanism {
            domain: "thermal".to_string(),
            mechanism_id: "thermal.thermal_zones".to_string(),
            description: format!("{thermal_zones} thermal zone(s) visible"),
            visibility_status: if thermal_zones > 0 {
                ContractEvidenceStatus::Measured
            } else {
                ContractEvidenceStatus::NotApplicableWithReason
            },
            control_status: ContractEvidenceStatus::NotControllable,
            evidence_refs: refs_for_kind(&pressure_refs, "thermal_pressure", inventory_ref.clone()),
            reason: "thermal zones are observation surfaces; adc-lab aborts bounded load instead of controlling thermal hardware".to_string(),
        },
        PlatformMechanism {
            domain: "memory".to_string(),
            mechanism_id: "memory.meminfo_vmstat_psi".to_string(),
            description: format!(
                "memory total observed as {} KiB with meminfo/vmstat/PSI pressure probes",
                memory_total.unwrap_or(0)
            ),
            visibility_status: if has_pressure(ResourcePressureKind::MemoryPressure)
                || memory_total.is_some()
            {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::Insufficient
            },
            control_status: ContractEvidenceStatus::MeasuredPartial,
            evidence_refs: refs_for_kind(&pressure_refs, "memory_pressure", inventory_ref.clone()),
            reason: "anonymous pressure is controllable within bounded user-space allocation limits; reclaim policy itself is platform-managed".to_string(),
        },
        PlatformMechanism {
            domain: "storage".to_string(),
            mechanism_id: "storage.tempfile_diskstats".to_string(),
            description: "bounded tempfile I/O and diskstats surface".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::StorageIo)),
            control_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::StorageIo)),
            evidence_refs: refs_for_kind(&pressure_refs, "storage_io", None),
            reason: "adc-lab controls temporary file size and removes the file after the probe; underlying storage/cache policy is platform-managed".to_string(),
        },
        PlatformMechanism {
            domain: "network".to_string(),
            mechanism_id: "network.proc_net_dev".to_string(),
            description: "network interface rx/tx counters and optional bounded endpoint attempt".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::NetworkIo)),
            control_status: ContractEvidenceStatus::MeasuredPartial,
            evidence_refs: refs_for_kind(&pressure_refs, "network_io", None),
            reason: "network counters are visible; bounded traffic generation depends on an available endpoint and otherwise records measured counter visibility".to_string(),
        },
        PlatformMechanism {
            domain: "scheduler_latency".to_string(),
            mechanism_id: "scheduler.monotonic_jitter_loop".to_string(),
            description: "target-local monotonic jitter loop".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::LatencyJitter)),
            control_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::LatencyJitter)),
            evidence_refs: refs_for_kind(&pressure_refs, "latency_jitter", None),
            reason: "adc-lab controls the loop interval and duration, but scheduler policy remains platform-managed".to_string(),
        },
        PlatformMechanism {
            domain: "observer".to_string(),
            mechanism_id: "observer.adc_lab_probe_overhead".to_string(),
            description: "adc-lab observation and artifact write overhead".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ObserverPressure)),
            control_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ObserverPressure)),
            evidence_refs: refs_for_kind(&pressure_refs, "observer_pressure", None),
            reason: "observer cadence and artifact bytes are bounded and recorded by the observer pressure probe".to_string(),
        },
    ];

    Ok(PlatformMechanismInventory {
        schema_version: "lab.platform_mechanism_inventory.v1".to_string(),
        target_id,
        target_class,
        mechanisms,
        evidence_refs,
        time_unix_ms: now_unix_ms(),
    })
}

pub fn boundary_probe_plan(target_id: String, target_class: String) -> BoundaryProbePlan {
    let probes = vec![
        boundary_probe(
            "cpu.governor_boundary",
            "CPU governor boundary",
            &["governor", "cpu_load_workers", "duration"],
            &[
                "frequency",
                "temperature",
                "worker_iterations",
                "latency_jitter",
            ],
            &["ambient_temperature", "background_daemons"],
            &["thermal abort threshold", "operator abort file"],
            &["restore governor when privileged control is used"],
            "governor-specific bounded CPU behavior",
            "all-frequency or sustained claims without controlled evidence",
        ),
        boundary_probe(
            "cpu.fixed_frequency_boundary",
            "fixed frequency boundary",
            &["frequency policy if controllable"],
            &["actual frequency", "temperature", "throughput"],
            &["firmware policy", "power supply", "ambient_temperature"],
            &["refuse without approved privileged control"],
            &["restore scaling min/max and governor"],
            "fixed-frequency behavior when policy is controllable",
            "fixed-frequency claims when platform or policy refuses control",
        ),
        boundary_probe(
            "thermal.sustained_boundary",
            "thermal boundary",
            &["worker_count", "duration", "abort_temp_c"],
            &["thermal_zones", "frequency", "abort_reason", "recovery"],
            &["cooling", "case", "ambient_temperature"],
            &["thermal abort", "policy duration ceiling"],
            &["cooldown observation"],
            "burst/sustained thermal margin under bounded load",
            "unbounded all-core default loops",
        ),
        boundary_probe(
            "memory.pressure_boundary",
            "memory pressure boundary",
            &["anonymous_memory_bytes", "duration"],
            &["meminfo", "vmstat", "PSI", "major_faults"],
            &["background memory use", "kernel reclaim policy"],
            &["memory byte ceiling", "process exit cleanup"],
            &["drop allocation by process exit"],
            "resident memory pressure side effects",
            "page-cache-dependent designs without reclaim evidence",
        ),
        boundary_probe(
            "storage.cache_coupling_boundary",
            "page cache / storage coupling boundary",
            &["tempfile_bytes", "read_write_mode"],
            &[
                "diskstats",
                "write_latency",
                "read_latency",
                "memory_available",
            ],
            &["filesystem", "device wear leveling", "background I/O"],
            &["storage byte ceiling", "tempfile cleanup"],
            &["remove tempfile and verify cleanup"],
            "bounded storage behavior and cache side effects",
            "continuous default writes without cadence evidence",
        ),
        boundary_probe(
            "network.io_boundary",
            "network I/O boundary",
            &["endpoint", "duration", "network_bytes"],
            &["interface_rx_tx", "latency", "cpu", "thermal"],
            &["LAN load", "Wi-Fi/Ethernet link state", "remote endpoint"],
            &["network byte ceiling", "connect timeout"],
            &["close socket"],
            "bounded network counter and latency side effects",
            "background upload/retry claims without endpoint evidence",
        ),
        boundary_probe(
            "latency.jitter_boundary",
            "latency / jitter boundary",
            &["loop_interval", "duration"],
            &["p50", "p95", "p99", "max"],
            &["scheduler class", "background daemons", "interrupt load"],
            &["duration ceiling"],
            &["no persistent state"],
            "real-time-ish claim boundary for observed pressure condition",
            "real-time-ish claims under unmeasured pressure",
        ),
        boundary_probe(
            "observer.effect_boundary",
            "observer effect boundary",
            &["sample_count", "artifact_bytes", "duration"],
            &["sample_overhead", "artifact_write_latency", "jitter_delta"],
            &["filesystem cache", "scheduler noise"],
            &["artifact byte ceiling", "tempfile cleanup"],
            &["remove observer temp artifact"],
            "safe default observation cadence",
            "low-overhead observer claims without measured overhead",
        ),
        boundary_probe(
            "recovery.boundary",
            "recovery boundary",
            &["cooldown_duration", "pressure_kind"],
            &["temperature", "frequency", "latency", "memory_available"],
            &["ambient_temperature", "background load"],
            &["stop pressure on abort"],
            &["verify cleanup and optional cooldown observation"],
            "degraded-mode exit criteria after pressure",
            "automatic recovery claims without post-pressure observation",
        ),
    ];
    BoundaryProbePlan {
        schema_version: "lab.boundary_probe_plan.v1".to_string(),
        plan_id: new_id("BOUNDARY-PLAN"),
        target_id,
        target_class,
        probes,
        time_unix_ms: now_unix_ms(),
    }
}

pub fn resource_coupling_report_for_run(
    run_dir: impl AsRef<Path>,
    target_id: String,
) -> LabResult<ResourceCouplingReport> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_run_dir(run_dir);
    let refs = pressure_result_refs(run_dir, &run_id)?;
    let has = |kind: &str| refs.contains_key(kind);
    let mut evidence_refs = refs.values().cloned().collect::<Vec<_>>();
    evidence_refs.sort();

    let chains = vec![
        coupling_chain(
            "memory.reclaim_to_storage_latency",
            "memory pressure",
            "reclaim/page-cache behavior",
            "storage latency and CPU wait side effects",
            "workload tail latency can increase when cache/reclaim competes with I/O",
            "process exit releases anonymous pressure; storage cache recovery remains platform-managed",
            status_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            refs_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
        ),
        coupling_chain(
            "storage.io_to_latency_thermal",
            "storage I/O",
            "filesystem and block-device scheduling",
            "CPU time, memory cache, and thermal side effects",
            "write/read latency can widen under cache or device pressure",
            "tempfile cleanup is verified; device/cache recovery needs follow-up observation",
            status_for_required(&refs, &["storage_io", "latency_jitter", "thermal_pressure"]),
            refs_for_required(&refs, &["storage_io", "latency_jitter", "thermal_pressure"]),
        ),
        coupling_chain(
            "cpu.load_to_thermal_frequency",
            "CPU pressure",
            "frequency governor and thermal management",
            "thermal margin and latency side effects",
            "sustained CPU work can reduce thermal margin and shift frequency behavior",
            "load stop plus cooldown observation required for sustained claims",
            status_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            refs_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
        ),
        coupling_chain(
            "network.io_to_cpu_latency",
            "network I/O",
            "network interface counters and TCP/connect behavior",
            "CPU, wakeup, and latency side effects",
            "background traffic or retries can consume CPU and widen latency tails",
            "socket close stops generated I/O; retry recovery requires endpoint-specific evidence",
            status_for_required(&refs, &["network_io", "latency_jitter"]),
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
        ),
        coupling_chain(
            "observer.to_workload_jitter",
            "observer pressure",
            "sampling and artifact writing",
            "scheduler, storage, and latency side effects",
            "observer cadence can perturb workload timing when artifact writes or sampling are too frequent",
            "default cadence must stay bounded and evidence-backed",
            status_for_required(&refs, &["observer_pressure", "latency_jitter"]),
            refs_for_required(&refs, &["observer_pressure", "latency_jitter"]),
        ),
    ];

    let missing = required_pressure_kinds()
        .iter()
        .filter(|kind| !has(kind))
        .map(|kind| format!("{kind} pressure result"))
        .collect::<Vec<_>>();
    let report_status = if missing.is_empty() {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::Insufficient
    };
    let next_evidence_needed = if missing.is_empty() {
        vec![
            "repeat trials under controlled governor/frequency states".to_string(),
            "extend thermal soak evidence within approved policy ceilings".to_string(),
        ]
    } else {
        missing.iter().map(|entry| format!("run {entry}")).collect()
    };

    Ok(ResourceCouplingReport {
        schema_version: "lab.resource_coupling_report.v1".to_string(),
        report_id: new_id("COUPLING"),
        target_id,
        report_status,
        chains,
        evidence_refs,
        unknowns: missing,
        next_evidence_needed,
        time_unix_ms: now_unix_ms(),
    })
}

pub fn target_operating_contract_for_run(
    run_dir: impl AsRef<Path>,
    target_id: String,
    target_class: String,
) -> LabResult<TargetOperatingContract> {
    let run_dir = run_dir.as_ref();
    let run_id = run_id_from_run_dir(run_dir);
    let refs = pressure_result_refs(run_dir, &run_id)?;
    let coupling_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/resource_coupling_report.json")?;
    let mut all_refs = refs.values().cloned().collect::<Vec<_>>();
    if let Some(reference) = coupling_ref.clone() {
        all_refs.push(reference);
    }
    all_refs.sort();

    let missing = required_pressure_kinds()
        .iter()
        .filter(|kind| !refs.contains_key(**kind))
        .map(|kind| format!("{kind} result"))
        .collect::<Vec<_>>();
    let contract_status = if missing.is_empty() {
        TargetOperatingContractStatus::MeasuredPartial
    } else {
        TargetOperatingContractStatus::Insufficient
    };

    let rules = vec![
        operating_rule(
            "cpu.sustained_all_core_requires_thermal_margin",
            OperatingRuleCategory::DegradedModeTrigger,
            "Sustained all-core CPU work must be bounded or degraded unless thermal soak evidence passes.",
            refs_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            ContractConfidence::Medium,
            &["bounded burst", "thermal degrade policy"],
            &["unbounded default all-core loop"],
        ),
        operating_rule(
            "memory.pressure_limits_storage_heavy_work",
            OperatingRuleCategory::DegradedModeTrigger,
            "Storage-heavy work must reduce cadence under memory pressure until reclaim/cache side effects are measured safe.",
            refs_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            ContractConfidence::Medium,
            &["bounded resident set", "coalesced writes", "drop or defer nonessential work"],
            &["page-cache-dependent default path without pressure evidence"],
        ),
        operating_rule(
            "storage.default_writes_must_be_bounded",
            OperatingRuleCategory::BurstOnly,
            "Default storage writes must be bounded and coalesced; sustained write cadence requires target-specific evidence.",
            refs_for_required(&refs, &["storage_io"]),
            ContractConfidence::Medium,
            &["bounded tempfile writes", "batched artifact writes"],
            &["continuous unbounded default logging"],
        ),
        operating_rule(
            "network.background_io_requires_backoff",
            OperatingRuleCategory::BurstOnly,
            "Background network I/O and retries require bounded cadence and backoff tied to observed CPU/latency side effects.",
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
            ContractConfidence::Low,
            &["bounded upload burst", "retry with backoff"],
            &["tight retry loop", "unbounded background upload"],
        ),
        operating_rule(
            "latency.real_time_claim_requires_pressure_jitter_evidence",
            OperatingRuleCategory::BlockedClaim,
            "Real-time-ish claims are blocked unless p95/p99/max jitter are measured under the relevant CPU, memory, storage, network, and observer conditions.",
            refs_for_required(&refs, &["latency_jitter"]),
            ContractConfidence::Medium,
            &["pressure-specific jitter budget", "degraded mode on tail widening"],
            &["generic real-time claim from idle-only evidence"],
        ),
        operating_rule(
            "observer.default_cadence_must_be_evidence_bounded",
            OperatingRuleCategory::AllowedDefault,
            "adc-lab observation is allowed by default only at measured bounded cadence and artifact volume.",
            refs_for_required(&refs, &["observer_pressure"]),
            ContractConfidence::Medium,
            &["bounded cadence", "bounded artifact bytes", "observer-off comparison"],
            &["high-frequency logging without observer-effect evidence"],
        ),
    ];

    let boundaries = vec![
        operating_boundary(
            "compute_thermal",
            "Compute and thermal claims are measured only for bounded load durations and observed frequency policy.",
            status_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            refs_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
        ),
        operating_boundary(
            "memory_cache_storage",
            "Memory/cache/storage coupling is measured as bounded smoke evidence, not a full resident-memory budget.",
            status_for_required(&refs, &["memory_pressure", "storage_io"]),
            refs_for_required(&refs, &["memory_pressure", "storage_io"]),
        ),
        operating_boundary(
            "network_latency",
            "Network rules are limited to visible interface counter and optional endpoint evidence.",
            status_for_required(&refs, &["network_io", "latency_jitter"]),
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
        ),
        operating_boundary(
            "observer_effect",
            "Observer overhead is measured only for adc-lab's bounded probe cadence and artifact write path.",
            status_for_required(&refs, &["observer_pressure"]),
            refs_for_required(&refs, &["observer_pressure"]),
        ),
    ];

    let next_evidence_needed = if missing.is_empty() {
        vec![
            "repeat pressure probes across governor states".to_string(),
            "run approved longer thermal soak within policy or update policy with explicit approval".to_string(),
            "run Pi5 reference target with the same contract suite".to_string(),
        ]
    } else {
        missing
            .iter()
            .map(|entry| format!("collect {entry}"))
            .collect()
    };

    Ok(TargetOperatingContract {
        schema_version: "lab.target_operating_contract.v1".to_string(),
        target_id,
        target_class,
        contract_status,
        rules,
        boundaries,
        unknowns: missing,
        next_evidence_needed,
        time_unix_ms: now_unix_ms(),
    })
}

pub fn pressure_result_refs(run_dir: &Path, run_id: &str) -> LabResult<BTreeMap<String, String>> {
    let mut refs = BTreeMap::new();
    let pressure_dir = run_dir.join("pressure");
    if !pressure_dir.exists() {
        return Ok(refs);
    }
    for entry in fs::read_dir(&pressure_dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.ends_with(".result.json") {
            continue;
        }
        let result: ResourcePressureResult = serde_json::from_slice(&fs::read(&path)?)?;
        let base_key = result.pressure_kind.as_str().to_string();
        let key = if refs.contains_key(&base_key) {
            format!(
                "{base_key}#{}",
                refs.keys().filter(|key| key.starts_with(&base_key)).count()
            )
        } else {
            base_key
        };
        refs.insert(key, artifact_uri_for_run(run_id, run_dir, path)?);
    }
    Ok(refs)
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

    let metrics = vec![
        metric(
            "anonymous_bytes_touched",
            Some(bytes as f64),
            "bytes",
            Some("bounded user-space anonymous allocation".to_string()),
        ),
        metric_delta(
            "mem_available_delta_kb",
            &before_mem,
            &after_mem,
            "MemAvailable",
            "KiB",
        ),
        metric_delta(
            "pgscan_delta",
            &before_vm,
            &after_vm,
            "pgscan_kswapd",
            "count",
        ),
        metric_delta(
            "pgsteal_delta",
            &before_vm,
            &after_vm,
            "pgsteal_kswapd",
            "count",
        ),
        metric_delta(
            "major_faults_delta",
            &before_vm,
            &after_vm,
            "pgmajfault",
            "count",
        ),
        metric(
            "memory_psi_some_avg10_delta",
            optional_delta_f64(before_psi, after_psi),
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

    Ok(result(
        target_id,
        ResourcePressureKind::MemoryPressure,
        ContractEvidenceStatus::MeasuredPartial,
        started.elapsed(),
        vec![factor("anonymous_memory_bytes", bytes.to_string())],
        vec![factor("meminfo", "before_after".to_string()), factor("vmstat", "before_after".to_string())],
        vec!["kernel reclaim policy".to_string(), "background memory activity".to_string()],
        metrics,
        vec![
            side_effect(
                "storage",
                ContractEvidenceStatus::MeasuredPartial,
                "memory pressure records reclaim/page-cache indicators; storage latency coupling requires paired storage evidence",
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
        vec!["bounded anonymous memory pressure was applied and released".to_string()],
        vec!["safe resident memory budget still requires laddered trials".to_string()],
    ))
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

    let metrics = vec![
        metric("bytes_written", Some(written as f64), "bytes", None),
        metric("bytes_read", Some(read_bytes as f64), "bytes", None),
        metric("write_latency_ms", Some(write_ms), "ms", None),
        metric("read_latency_ms", Some(read_ms), "ms", None),
        metric(
            "disk_read_sectors_delta",
            optional_delta_u64(before_disk.map(|v| v.0), after_disk.map(|v| v.0)),
            "sectors",
            None,
        ),
        metric(
            "disk_write_sectors_delta",
            optional_delta_u64(before_disk.map(|v| v.1), after_disk.map(|v| v.1)),
            "sectors",
            None,
        ),
        metric_delta(
            "mem_available_delta_kb",
            &before_mem,
            &after_mem,
            "MemAvailable",
            "KiB",
        ),
    ];

    Ok(result(
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
        vec!["bounded tempfile write/read latency was measured".to_string()],
        vec!["sustained storage cadence and flash-wear claims require longer evidence".to_string()],
    ))
}

fn run_network_io(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let before = netdev_totals();
    let mut endpoint_status = "not_configured".to_string();
    let mut connect_latency_ms = None;
    if let Some(endpoint) = options.network_endpoint.as_ref() {
        let connect_started = Instant::now();
        match resolve_socket_addr(endpoint).and_then(|addr| {
            TcpStream::connect_timeout(&addr, Duration::from_millis(500)).map_err(LabError::from)
        }) {
            Ok(stream) => {
                endpoint_status = "connected".to_string();
                connect_latency_ms = Some(connect_started.elapsed().as_secs_f64() * 1000.0);
                drop(stream);
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
    let status = if before.is_some() && after.is_some() {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::NotApplicableWithReason
    };

    let metrics = vec![
        metric("rx_bytes_delta", rx_delta, "bytes", None),
        metric("tx_bytes_delta", tx_delta, "bytes", None),
        metric(
            "connect_latency_ms",
            connect_latency_ms,
            "ms",
            Some(endpoint_status.clone()),
        ),
    ];

    Ok(result(
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
                ContractEvidenceStatus::MeasuredPartial,
                "network counter polling and optional connect attempt execute on CPU",
                Vec::new(),
            ),
            side_effect(
                "latency",
                ContractEvidenceStatus::MeasuredPartial,
                "connect latency is recorded when an endpoint is configured",
                Vec::new(),
            ),
        ],
        safety(
            options,
            true,
            vec!["closed optional TCP socket".to_string()],
        ),
        vec!["network interface rx/tx counters were observed".to_string()],
        vec![
            "background upload cadence and retry/backoff require endpoint-specific trials"
                .to_string(),
        ],
    ))
}

fn run_latency_jitter(
    target_id: String,
    options: &PressureProbeOptions,
) -> LabResult<ResourcePressureResult> {
    let started = Instant::now();
    let samples = jitter_samples(options.duration, Duration::from_millis(1));
    let metrics = jitter_metrics(&samples);
    Ok(result(
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
        vec!["idle/current-condition jitter distribution was measured".to_string()],
        vec!["pressure-specific p95/p99 claims require paired pressure runs".to_string()],
    ))
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
    Ok(result(
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
    ))
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

    Ok(result(
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
    ))
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
    ResourcePressureResult {
        schema_version: "lab.resource_pressure_result.v1".to_string(),
        result_id: new_id("PRESSURE"),
        target_id,
        pressure_kind,
        status,
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
        time_unix_ms: now_unix_ms(),
    }
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

fn metric_delta(
    metric_id: &str,
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
    key: &str,
    unit: &str,
) -> ResourceMetric {
    metric(
        metric_id,
        optional_delta_u64(before.get(key).copied(), after.get(key).copied()),
        unit,
        None,
    )
}

fn optional_delta_u64(before: Option<u64>, after: Option<u64>) -> Option<f64> {
    Some(after? as f64 - before? as f64)
}

fn optional_delta_f64(before: Option<f64>, after: Option<f64>) -> Option<f64> {
    Some(after? - before?)
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

fn read_json_if_exists<T: serde::de::DeserializeOwned>(path: &Path) -> LabResult<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
}

fn artifact_ref_if_exists(
    run_dir: &Path,
    run_id: &str,
    relative_path: &str,
) -> LabResult<Option<String>> {
    let path = run_dir.join(relative_path);
    if path.exists() {
        artifact_uri_for_run(run_id, run_dir, path).map(Some)
    } else {
        Ok(None)
    }
}

fn pressure_kind_set(run_dir: &Path) -> LabResult<BTreeSet<String>> {
    Ok(
        pressure_result_refs(run_dir, &run_id_from_run_dir(run_dir))?
            .keys()
            .cloned()
            .collect(),
    )
}

fn surface_available(inventory: &TargetInventory, surface_id: &str) -> bool {
    inventory
        .control_surfaces
        .iter()
        .any(|surface| surface.surface_id == surface_id && surface.available)
}

fn refs_for_kind(
    refs: &BTreeMap<String, String>,
    kind: &str,
    fallback: Option<String>,
) -> Vec<String> {
    refs.get(kind)
        .cloned()
        .into_iter()
        .chain(
            refs.iter()
                .filter(|(key, _)| key.starts_with(&format!("{kind}#")))
                .map(|(_, value)| value.clone()),
        )
        .chain(fallback)
        .collect()
}

fn status_for_pressure_presence(present: bool) -> ContractEvidenceStatus {
    if present {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::Insufficient
    }
}

fn required_pressure_kinds() -> &'static [&'static str] {
    &[
        "cpu_pressure",
        "thermal_pressure",
        "memory_pressure",
        "storage_io",
        "network_io",
        "latency_jitter",
        "observer_pressure",
    ]
}

fn status_for_required(
    refs: &BTreeMap<String, String>,
    required: &[&str],
) -> ContractEvidenceStatus {
    if required.iter().all(|kind| refs.contains_key(*kind)) {
        ContractEvidenceStatus::MeasuredPartial
    } else {
        ContractEvidenceStatus::Insufficient
    }
}

fn refs_for_required(refs: &BTreeMap<String, String>, required: &[&str]) -> Vec<String> {
    required
        .iter()
        .flat_map(|kind| refs_for_kind(refs, kind, None))
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn boundary_probe(
    probe_id: &str,
    boundary: &str,
    controlled_factors: &[&str],
    observed_covariates: &[&str],
    uncontrolled_confounders: &[&str],
    safety_abort_condition: &[&str],
    restore_cleanup: &[&str],
    claim_supported: &str,
    claim_blocked: &str,
) -> BoundaryProbe {
    BoundaryProbe {
        probe_id: probe_id.to_string(),
        boundary: boundary.to_string(),
        controlled_factors: controlled_factors
            .iter()
            .map(|value| value.to_string())
            .collect(),
        observed_covariates: observed_covariates
            .iter()
            .map(|value| value.to_string())
            .collect(),
        uncontrolled_confounders: uncontrolled_confounders
            .iter()
            .map(|value| value.to_string())
            .collect(),
        safety_abort_condition: safety_abort_condition
            .iter()
            .map(|value| value.to_string())
            .collect(),
        restore_cleanup: restore_cleanup
            .iter()
            .map(|value| value.to_string())
            .collect(),
        claim_supported: claim_supported.to_string(),
        claim_blocked: claim_blocked.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn coupling_chain(
    chain_id: &str,
    pressure: &str,
    platform_response: &str,
    secondary_pressure: &str,
    performance_degradation: &str,
    recovery_behavior: &str,
    status: ContractEvidenceStatus,
    evidence_refs: Vec<String>,
) -> ResourceCouplingChain {
    ResourceCouplingChain {
        chain_id: chain_id.to_string(),
        pressure: pressure.to_string(),
        platform_response: platform_response.to_string(),
        secondary_pressure: secondary_pressure.to_string(),
        performance_degradation: performance_degradation.to_string(),
        recovery_behavior: recovery_behavior.to_string(),
        status,
        evidence_refs,
    }
}

fn operating_rule(
    rule_id: &str,
    category: OperatingRuleCategory,
    statement: &str,
    evidence_refs: Vec<String>,
    confidence: ContractConfidence,
    allowed_design: &[&str],
    blocked_design: &[&str],
) -> OperatingContractRule {
    OperatingContractRule {
        rule_id: rule_id.to_string(),
        category,
        statement: statement.to_string(),
        evidence_refs,
        confidence,
        allowed_design: allowed_design
            .iter()
            .map(|value| value.to_string())
            .collect(),
        blocked_design: blocked_design
            .iter()
            .map(|value| value.to_string())
            .collect(),
    }
}

fn operating_boundary(
    boundary_id: &str,
    statement: &str,
    status: ContractEvidenceStatus,
    evidence_refs: Vec<String>,
) -> OperatingBoundary {
    OperatingBoundary {
        boundary_id: boundary_id.to_string(),
        statement: statement.to_string(),
        status,
        evidence_refs,
    }
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
            ContractEvidenceStatus::MeasuredPartial
                | ContractEvidenceStatus::NotApplicableWithReason
        ));
    }

    #[test]
    fn contract_validation_boundary_plan_covers_required_probe_surfaces() {
        let plan = boundary_probe_plan("target55".to_string(), "raspberry_pi_4".to_string());
        let ids = plan
            .probes
            .iter()
            .map(|probe| probe.probe_id.as_str())
            .collect::<BTreeSet<_>>();
        for required in [
            "cpu.governor_boundary",
            "cpu.fixed_frequency_boundary",
            "thermal.sustained_boundary",
            "memory.pressure_boundary",
            "storage.cache_coupling_boundary",
            "network.io_boundary",
            "latency.jitter_boundary",
            "observer.effect_boundary",
            "recovery.boundary",
        ] {
            assert!(ids.contains(required), "missing {required}");
        }
    }
}
