use crate::contracts::{
    ApprovalRecord, BoundaryProbe, BoundaryProbePlan, ContractConfidence, ContractEvidenceGap,
    ContractEvidenceStatus, ContractFactor, ControlPlan, ControlResult, ControlResultStatus,
    NetworkPressureEvidence, NetworkPressureMode, OperatingBoundary, OperatingContractRule,
    OperatingRuleCategory, OperatingRuleSource, PlatformMechanism, PlatformMechanismInventory,
    PressureCondition, PressureEffect, PressureIntensity, PressureSafety, ResourceCouplingChain,
    ResourceCouplingEvidenceClass, ResourceCouplingReport, ResourceMetric,
    ResourcePressureEvidenceClass, ResourcePressureKind, ResourcePressureResult,
    ResourceSideEffect, RestoreAttemptStatus, RestoreLease, RestoreStatus, TargetInventory,
    TargetOperatingContract, TargetOperatingContractStatus,
};
use crate::control::{approval_matches, CPUFREQ_SET_GOVERNOR};
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
    let pressure_results = pressure_results_by_kind(run_dir)?;
    let pressure_kinds = pressure_kind_set(run_dir)?;
    let has_pressure = |kind: ResourcePressureKind| pressure_kinds.contains(kind.as_str());
    let memory_pressure_effect_observed = pressure_results
        .get("memory_pressure")
        .into_iter()
        .flatten()
        .any(|result| result.pressure_effect.observed);
    let network_pressure_status = pressure_results
        .get("network_io")
        .and_then(|results| results.first())
        .map(|result| result.status.clone())
        .unwrap_or(ContractEvidenceStatus::Insufficient);

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
    let cpufreq_control_refs = cpufreq_control_evidence_refs(run_dir, &run_id)?;
    let cpufreq_control_observed = !cpufreq_control_refs.is_empty();
    let cpufreq_evidence_refs = inventory_ref
        .clone()
        .into_iter()
        .chain(cpufreq_control_refs.clone())
        .collect::<Vec<_>>();

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
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: ContractEvidenceStatus::NotApplicableWithReason,
            evidence_status: if inventory.is_some() {
                ContractEvidenceStatus::Measured
            } else {
                ContractEvidenceStatus::Insufficient
            },
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
            platform_control_status: if cpufreq_control_observed {
                ContractEvidenceStatus::MeasuredPartial
            } else if cpufreq_surface {
                ContractEvidenceStatus::Insufficient
            } else {
                ContractEvidenceStatus::NotControllable
            },
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::CpuPressure)),
            evidence_status: if cpufreq_policies > 0 {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::Insufficient
            },
            evidence_refs: cpufreq_evidence_refs,
            reason: if cpufreq_control_observed {
                "cpufreq approved apply/verify/restore control evidence was found in this run".to_string()
            } else if cpufreq_surface {
                "cpufreq sysfs surface is visible, but no approved apply/verify/restore control result was found in this run".to_string()
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
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ThermalPressure)),
            evidence_status: if thermal_zones > 0
                && has_pressure(ResourcePressureKind::ThermalPressure)
            {
                ContractEvidenceStatus::MeasuredPartial
            } else if thermal_zones > 0 {
                ContractEvidenceStatus::Insufficient
            } else {
                ContractEvidenceStatus::NotApplicableWithReason
            },
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
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::MemoryPressure)),
            evidence_status: if memory_pressure_effect_observed {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::Insufficient
            },
            evidence_refs: refs_for_kind(&pressure_refs, "memory_pressure", inventory_ref.clone()),
            reason: "meminfo/vmstat/PSI are observation surfaces; adc-lab can inject bounded anonymous allocation, but Linux reclaim/page-cache policy is platform-managed and not controlled".to_string(),
        },
        PlatformMechanism {
            domain: "storage".to_string(),
            mechanism_id: "storage.tempfile_diskstats".to_string(),
            description: "bounded tempfile I/O and diskstats surface".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::StorageIo)),
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::StorageIo)),
            evidence_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::StorageIo)),
            evidence_refs: refs_for_kind(&pressure_refs, "storage_io", None),
            reason: "adc-lab controls temporary file size and cleanup only; storage scheduler, filesystem cache, media wear, and persistence policy remain platform-managed".to_string(),
        },
        PlatformMechanism {
            domain: "network".to_string(),
            mechanism_id: "network.proc_net_dev".to_string(),
            description: "network interface rx/tx counters and optional bounded endpoint attempt".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::NetworkIo)),
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: network_pressure_status.clone(),
            evidence_status: network_pressure_status,
            evidence_refs: refs_for_kind(&pressure_refs, "network_io", None),
            reason: "network counters are visible when /proc/net/dev is available; route/link behavior and endpoint availability are external conditions, not adc-lab-controlled platform mechanisms".to_string(),
        },
        PlatformMechanism {
            domain: "scheduler_latency".to_string(),
            mechanism_id: "scheduler.monotonic_jitter_loop".to_string(),
            description: "target-local monotonic jitter loop".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::LatencyJitter)),
            platform_control_status: ContractEvidenceStatus::NotControllable,
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::LatencyJitter)),
            evidence_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::LatencyJitter)),
            evidence_refs: refs_for_kind(&pressure_refs, "latency_jitter", None),
            reason: "adc-lab controls the loop interval and duration, but scheduler policy remains platform-managed".to_string(),
        },
        PlatformMechanism {
            domain: "observer".to_string(),
            mechanism_id: "observer.adc_lab_probe_overhead".to_string(),
            description: "adc-lab observation and artifact write overhead".to_string(),
            visibility_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ObserverPressure)),
            platform_control_status: ContractEvidenceStatus::MeasuredPartial,
            pressure_probe_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ObserverPressure)),
            evidence_status: status_for_pressure_presence(has_pressure(ResourcePressureKind::ObserverPressure)),
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
            coupling_status_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            coupling_class_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            refs_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            vec![evidence_gap(
                "individual memory, storage, and jitter artifacts are ingredients only; no simultaneous or phased memory+storage+jitter scenario was run",
                "phase-based memory+storage+jitter coupling probe",
                &["baseline storage latency", "memory pressure only", "storage I/O under memory pressure", "recovery phase"],
                "run a composite boundary probe before marking memory-to-storage coupling measured",
                "adc-lab",
            )],
        ),
        coupling_chain(
            "storage.io_to_latency_thermal",
            "storage I/O",
            "filesystem and block-device scheduling",
            "CPU time, memory cache, and thermal side effects",
            "write/read latency can widen under cache or device pressure",
            "tempfile cleanup is verified; device/cache recovery needs follow-up observation",
            coupling_status_for_required(&refs, &["storage_io", "latency_jitter", "thermal_pressure"]),
            coupling_class_for_required(&refs, &["storage_io", "latency_jitter", "thermal_pressure"]),
            refs_for_required(&refs, &["storage_io", "latency_jitter", "thermal_pressure"]),
            vec![evidence_gap(
                "storage, latency, and thermal artifacts were not collected under one controlled storage pressure scenario",
                "phase-based storage+latency+thermal coupling probe",
                &["baseline jitter", "storage I/O while jitter loop runs", "thermal side-effect observation", "recovery phase"],
                "run a composite storage coupling probe before claiming storage-induced latency/thermal degradation",
                "adc-lab",
            )],
        ),
        coupling_chain(
            "cpu.load_to_thermal_frequency",
            "CPU pressure",
            "frequency governor and thermal management",
            "thermal margin and latency side effects",
            "sustained CPU work can reduce thermal margin and shift frequency behavior",
            "load stop plus cooldown observation required for sustained claims",
            coupling_status_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            coupling_class_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            refs_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            vec![evidence_gap(
                "CPU and thermal artifacts are bounded ingredients; sustained frequency/thermal coupling needs repeated soak phases",
                "bounded CPU thermal soak with cooldown/recovery phases",
                &["governor-controlled load ladder", "5/15/30 minute soak when approved", "cooldown curve"],
                "run approved repeated soak probes before marking sustained CPU-thermal coupling measured",
                "adc-lab",
            )],
        ),
        coupling_chain(
            "network.io_to_cpu_latency",
            "network I/O",
            "network interface counters and TCP/connect behavior",
            "CPU, wakeup, and latency side effects",
            "background traffic or retries can consume CPU and widen latency tails",
            "socket close stops generated I/O; retry recovery requires endpoint-specific evidence",
            coupling_status_for_required(&refs, &["network_io", "latency_jitter"]),
            coupling_class_for_required(&refs, &["network_io", "latency_jitter"]),
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
            vec![evidence_gap(
                "network and jitter artifacts do not prove network-induced latency without endpoint transfer and paired jitter measurement",
                "bounded network transfer plus concurrent jitter probe",
                &["endpoint availability", "generated bytes", "jitter under transfer", "retry/backoff behavior"],
                "run endpoint-backed bounded transfer before claiming network coupling measured",
                "operator_approval",
            )],
        ),
        coupling_chain(
            "observer.to_workload_jitter",
            "observer pressure",
            "sampling and artifact writing",
            "scheduler, storage, and latency side effects",
            "observer cadence can perturb workload timing when artifact writes or sampling are too frequent",
            "default cadence must stay bounded and evidence-backed",
            coupling_status_for_required(&refs, &["observer_pressure", "latency_jitter"]),
            coupling_class_for_required(&refs, &["observer_pressure", "latency_jitter"]),
            refs_for_required(&refs, &["observer_pressure", "latency_jitter"]),
            vec![evidence_gap(
                "observer and jitter artifacts are not a workload-specific observer-effect proof",
                "observer-off/on workload jitter probe",
                &["workload baseline", "observer-on workload jitter", "artifact write cadence", "recovery phase"],
                "run observer-off/on around the actual workload before allowing low-overhead observer claims",
                "adc-lab",
            )],
        ),
    ];

    let missing = required_pressure_kinds()
        .iter()
        .filter(|kind| !has(kind))
        .map(|kind| format!("{kind} pressure result"))
        .collect::<Vec<_>>();
    let report_status = ContractEvidenceStatus::Insufficient;
    let mut unknowns = missing;
    unknowns.push(
        "composite resource-coupling phases were not run; individual pressure artifacts are evidence ingredients only".to_string(),
    );
    let mut next_evidence_needed = unknowns
        .iter()
        .map(|entry| format!("resolve {entry}"))
        .collect::<Vec<_>>();
    next_evidence_needed.push(
        "run baseline -> pressure -> paired pressure -> recovery scenarios before marking coupling measured".to_string(),
    );

    Ok(ResourceCouplingReport {
        schema_version: "lab.resource_coupling_report.v1".to_string(),
        report_id: new_id("COUPLING"),
        target_id,
        report_status,
        chains,
        evidence_refs,
        unknowns,
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
    let pressure_results = pressure_results_by_kind(run_dir)?;
    let network_boundary_measured = pressure_results
        .get("network_io")
        .into_iter()
        .flatten()
        .any(|result| {
            result.network_evidence.as_ref().is_some_and(|evidence| {
                matches!(evidence.network_mode, NetworkPressureMode::BoundedTransfer)
            })
        });
    let memory_pressure_effect_observed = pressure_results
        .get("memory_pressure")
        .into_iter()
        .flatten()
        .any(|result| result.pressure_effect.observed);
    let coupling_ref =
        artifact_ref_if_exists(run_dir, &run_id, "reports/resource_coupling_report.json")?;
    let coupling_report = read_json_if_exists::<ResourceCouplingReport>(
        &run_dir.join("reports/resource_coupling_report.json"),
    )?;
    let composite_coupling_measured = coupling_report.as_ref().is_some_and(|report| {
        report.chains.iter().any(|chain| {
            matches!(
                chain.coupling_evidence_class,
                ResourceCouplingEvidenceClass::CompositeMeasured
            )
        })
    });
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
    let mut unknowns = missing.clone();
    if !composite_coupling_measured {
        unknowns.push(
            "composite resource coupling not measured; pressure artifacts are ingredients only"
                .to_string(),
        );
    }
    if !memory_pressure_effect_observed {
        unknowns.push("memory pressure effect not observed; anonymous allocation may only be allocation smoke".to_string());
    }
    if !network_boundary_measured {
        unknowns.push("network I/O boundary not measured by bounded transfer".to_string());
    }
    let contract_status = if missing.is_empty()
        && composite_coupling_measured
        && memory_pressure_effect_observed
        && network_boundary_measured
    {
        TargetOperatingContractStatus::MeasuredPartial
    } else {
        TargetOperatingContractStatus::Insufficient
    };

    let rules = vec![
        operating_rule(
            "cpu.sustained_all_core_requires_thermal_margin",
            OperatingRuleCategory::DegradedModeTrigger,
            "Sustained all-core CPU work must be bounded or degraded unless thermal soak evidence passes.",
            OperatingRuleSource::EvidenceNeededRule,
            "Bounded CPU and thermal artifacts show the short-run probe surface, but sustained all-core behavior requires repeated soak and cooldown evidence before this becomes a measured target rule.",
            refs_for_required(&refs, &["cpu_pressure", "thermal_pressure"]),
            ContractConfidence::Low,
            &["bounded burst", "thermal degrade policy"],
            &["unbounded default all-core loop"],
        ),
        operating_rule(
            "memory.pressure_limits_storage_heavy_work",
            OperatingRuleCategory::DegradedModeTrigger,
            "Storage-heavy work must reduce cadence under memory pressure until reclaim/cache side effects are measured safe.",
            OperatingRuleSource::EvidenceNeededRule,
            "Memory, storage, and jitter artifacts are separate ingredients; no paired memory+storage pressure phase proves the coupling yet.",
            refs_for_required(&refs, &["memory_pressure", "storage_io", "latency_jitter"]),
            ContractConfidence::Low,
            &["bounded resident set", "coalesced writes", "drop or defer nonessential work"],
            &["page-cache-dependent default path without pressure evidence"],
        ),
        operating_rule(
            "storage.default_writes_must_be_bounded",
            OperatingRuleCategory::BurstOnly,
            "Default storage writes must be bounded and coalesced; sustained write cadence requires target-specific evidence.",
            OperatingRuleSource::GenericLabRule,
            "The bounded tempfile probe can verify cleanup and short-path latency only; the bounded-write rule is a lab safety rule, not a target-specific flash-wear or sustained-cadence result.",
            refs_for_required(&refs, &["storage_io"]),
            ContractConfidence::Low,
            &["bounded tempfile writes", "batched artifact writes"],
            &["continuous unbounded default logging"],
        ),
        operating_rule(
            "network.background_io_requires_backoff",
            OperatingRuleCategory::BurstOnly,
            "Background network I/O and retries require bounded cadence and backoff tied to observed CPU/latency side effects.",
            OperatingRuleSource::EvidenceNeededRule,
            "Counter-only or endpoint-attempt network evidence does not measure bounded transfer, retry, loss, CPU side effect, or latency side effect.",
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
            ContractConfidence::Low,
            &["bounded upload burst", "retry with backoff"],
            &["tight retry loop", "unbounded background upload"],
        ),
        operating_rule(
            "latency.real_time_claim_requires_pressure_jitter_evidence",
            OperatingRuleCategory::BlockedClaim,
            "Real-time-ish claims are blocked unless p95/p99/max jitter are measured under the relevant CPU, memory, storage, network, and observer conditions.",
            OperatingRuleSource::GenericLabRule,
            "Current-condition jitter evidence is useful as a baseline, but pressure-specific tail latency must be measured under the matching pressure condition.",
            refs_for_required(&refs, &["latency_jitter"]),
            ContractConfidence::Low,
            &["pressure-specific jitter budget", "degraded mode on tail widening"],
            &["generic real-time claim from idle-only evidence"],
        ),
        operating_rule(
            "observer.default_cadence_must_be_evidence_bounded",
            OperatingRuleCategory::AllowedDefault,
            "adc-lab observation is allowed by default only at measured bounded cadence and artifact volume.",
            OperatingRuleSource::EvidenceNeededRule,
            "The observer probe records a bounded observer-off/on smoke; default cadence for real workloads still needs workload-specific observer-effect evidence.",
            refs_for_required(&refs, &["observer_pressure"]),
            ContractConfidence::Low,
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
            vec![evidence_gap(
                "sustained all-core thermal margin is not proven by short bounded probes",
                "repeated CPU thermal soak with cooldown curve",
                &["5/15/30 minute soak", "cooldown/recovery curve", "governor/frequency condition"],
                "run approved thermal soak probes before sustained-design claims",
                "operator_approval",
            )],
        ),
        operating_boundary(
            "memory_cache_storage",
            "Memory/cache/storage coupling is not measured by separate bounded smoke artifacts.",
            ContractEvidenceStatus::Insufficient,
            refs_for_required(&refs, &["memory_pressure", "storage_io"]),
            vec![evidence_gap(
                "memory pressure effect and storage latency under that pressure need paired phases",
                "memory pressure + storage I/O + jitter composite probe",
                &["pressure_effect_observed", "storage latency under memory pressure", "recovery behavior"],
                "run paired memory/storage boundary probe",
                "adc-lab",
            )],
        ),
        operating_boundary(
            "network_latency",
            "Network rules are limited to visible interface counters unless a bounded transfer endpoint is available.",
            if network_boundary_measured {
                ContractEvidenceStatus::MeasuredPartial
            } else {
                ContractEvidenceStatus::Insufficient
            },
            refs_for_required(&refs, &["network_io", "latency_jitter"]),
            vec![evidence_gap(
                "network counter-only or endpoint-attempt evidence is not a network I/O boundary measurement",
                "bounded endpoint transfer plus latency/jitter side-effect probe",
                &["endpoint_available", "traffic_generated_bytes", "latency under transfer", "retry/backoff behavior"],
                "run endpoint-backed transfer before allowing network background I/O design rules",
                "operator_approval",
            )],
        ),
        operating_boundary(
            "observer_effect",
            "Observer overhead is measured only for adc-lab's bounded probe cadence and artifact write path.",
            status_for_required(&refs, &["observer_pressure"]),
            refs_for_required(&refs, &["observer_pressure"]),
            vec![evidence_gap(
                "workload-specific observer effect remains unmeasured",
                "observer-off/on workload probe",
                &["workload baseline", "observer-on jitter", "artifact cadence"],
                "measure observer cadence around the target workload before low-overhead claims",
                "adc-lab",
            )],
        ),
    ];

    let next_evidence_needed = if unknowns.is_empty() {
        vec![
            "repeat pressure probes across governor states".to_string(),
            "run approved longer thermal soak within policy or update policy with explicit approval".to_string(),
            "run Pi5 reference target with the same contract suite".to_string(),
        ]
    } else {
        unknowns
            .iter()
            .map(|entry| format!("collect evidence to resolve: {entry}"))
            .collect()
    };

    Ok(TargetOperatingContract {
        schema_version: "lab.target_operating_contract.v1".to_string(),
        target_id,
        target_class,
        contract_status,
        rules,
        boundaries,
        unknowns,
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

fn pressure_results_by_kind(
    run_dir: &Path,
) -> LabResult<BTreeMap<String, Vec<ResourcePressureResult>>> {
    let mut results: BTreeMap<String, Vec<ResourcePressureResult>> = BTreeMap::new();
    let pressure_dir = run_dir.join("pressure");
    if !pressure_dir.exists() {
        return Ok(results);
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
        results
            .entry(result.pressure_kind.as_str().to_string())
            .or_default()
            .push(result);
    }
    Ok(results)
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
    let network_mode = if options.network_endpoint.is_some() {
        NetworkPressureMode::EndpointAttempt
    } else {
        NetworkPressureMode::CounterOnly
    };
    if let Some(endpoint) = options.network_endpoint.as_ref() {
        let connect_started = Instant::now();
        match resolve_socket_addr(endpoint).and_then(|addr| {
            TcpStream::connect_timeout(&addr, Duration::from_millis(500)).map_err(LabError::from)
        }) {
            Ok(stream) => {
                endpoint_status = "connected".to_string();
                endpoint_available = true;
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
    let status = if options.network_endpoint.is_none() {
        ContractEvidenceStatus::NotApplicableWithReason
    } else if endpoint_available && before.is_some() && after.is_some() {
        ContractEvidenceStatus::MeasuredPartial
    } else if before.is_some() && after.is_some() {
        ContractEvidenceStatus::Insufficient
    } else {
        ContractEvidenceStatus::NotApplicableWithReason
    };
    let generated_bytes = 0u64;
    let pressure_effect_observed = endpoint_available && connect_latency_ms.is_some();

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
        if options.network_endpoint.is_some() {
            vec!["network endpoint attempt completed; no bounded transfer was generated".to_string()]
        } else {
            vec!["network interface rx/tx counters were observed; network I/O boundary was not measured without an endpoint".to_string()]
        },
        vec![
            "background upload cadence and retry/backoff require endpoint-specific trials"
                .to_string(),
            "bounded transfer, packet loss, and retry behavior are not measured by counter-only evidence".to_string(),
        ],
    );
    result.evidence_class = if options.network_endpoint.is_some() {
        ResourcePressureEvidenceClass::Smoke
    } else {
        ResourcePressureEvidenceClass::NotApplicable
    };
    result.intensity = PressureIntensity {
        requested: format!("network_bytes_max={}", options.network_bytes),
        relative_to_target: if options.network_endpoint.is_some() {
            "endpoint attempt only; no bounded transfer generated".to_string()
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

#[derive(Debug)]
struct ArtifactEntry<T> {
    artifact_ref: String,
    value: T,
}

fn cpufreq_control_evidence_refs(run_dir: &Path, run_id: &str) -> LabResult<Vec<String>> {
    let plans = read_json_entries::<ControlPlan>(run_dir, run_id, "plans", |name| {
        name.ends_with(".json") && !name.ends_with(".result.json")
    })?;
    let approvals = read_json_entries::<ApprovalRecord>(run_dir, run_id, "approvals", |name| {
        name.ends_with(".json")
    })?;
    let results = read_json_entries::<ControlResult>(run_dir, run_id, "plans", |name| {
        name.ends_with(".result.json")
    })?;
    let leases = read_json_entries::<RestoreLease>(run_dir, run_id, "leases", |name| {
        name.ends_with(".json")
    })?;
    let Some(health_ref) = restore_health_check_ref(run_dir, run_id)? else {
        return Ok(Vec::new());
    };

    for plan in plans.iter().filter(|entry| {
        entry.value.operation.operation_id == CPUFREQ_SET_GOVERNOR
            && entry.value.approval_required
            && entry.value.restore_required
    }) {
        let Some(approval) = approvals.iter().find(|entry| {
            entry.value.approved_plan_id == plan.value.plan_id
                && entry.value.approved_operation.operation_id == CPUFREQ_SET_GOVERNOR
                && entry.value.restore_required
                && entry
                    .value
                    .approved_actions
                    .iter()
                    .any(|action| action == CPUFREQ_SET_GOVERNOR)
        }) else {
            continue;
        };
        if approval_matches(&plan.value, &approval.value).is_err() {
            continue;
        }
        let Some(applied) = results.iter().find(|entry| {
            entry.value.plan_id == plan.value.plan_id
                && entry.value.operation_id == CPUFREQ_SET_GOVERNOR
                && matches!(entry.value.status, ControlResultStatus::Applied)
                && entry.value.restore_lease.is_some()
        }) else {
            continue;
        };
        let Some(lease_id) = applied
            .value
            .restore_lease
            .as_ref()
            .map(|lease| lease.lease_id.as_str())
        else {
            continue;
        };
        let Some(lease) = leases.iter().find(|entry| {
            entry.value.lease_id == lease_id
                && entry.value.operation_id == CPUFREQ_SET_GOVERNOR
                && matches!(entry.value.restore_status, RestoreStatus::Restored)
        }) else {
            continue;
        };
        let Some(restored) = results.iter().find(|entry| {
            entry.value.plan_id == plan.value.plan_id
                && entry.value.operation_id == CPUFREQ_SET_GOVERNOR
                && matches!(entry.value.status, ControlResultStatus::Restored)
                && entry.value.restore_attempted
                && entry.value.restore_result.as_ref().is_some_and(|restore| {
                    matches!(restore.status, RestoreAttemptStatus::Succeeded)
                })
                && entry
                    .value
                    .restore_lease
                    .as_ref()
                    .is_some_and(|restore_lease| {
                        restore_lease.lease_id == lease_id
                            && matches!(restore_lease.restore_status, RestoreStatus::Restored)
                    })
        }) else {
            continue;
        };

        let mut refs = vec![
            plan.artifact_ref.clone(),
            approval.artifact_ref.clone(),
            applied.artifact_ref.clone(),
            lease.artifact_ref.clone(),
            restored.artifact_ref.clone(),
            health_ref.clone(),
        ];
        refs.sort();
        refs.dedup();
        return Ok(refs);
    }

    Ok(Vec::new())
}

fn read_json_entries<T>(
    run_dir: &Path,
    run_id: &str,
    relative_dir: &str,
    include: impl Fn(&str) -> bool,
) -> LabResult<Vec<ArtifactEntry<T>>>
where
    T: serde::de::DeserializeOwned,
{
    let dir = run_dir.join(relative_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !include(name) {
            continue;
        }
        entries.push(ArtifactEntry {
            artifact_ref: artifact_uri_for_run(run_id, run_dir, &path)?,
            value: serde_json::from_slice(&fs::read(&path)?)?,
        });
    }
    Ok(entries)
}

fn restore_health_check_ref(run_dir: &Path, run_id: &str) -> LabResult<Option<String>> {
    let relative_path = "health/restore_health_check.json";
    let path = run_dir.join(relative_path);
    if !path.exists() {
        return Ok(None);
    }
    let value: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
    let healthy = value
        .get("schema_version")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value == "lab.health_check.v1")
        && value
            .get("status")
            .and_then(|value| value.as_str())
            .is_some_and(|value| value == "ok");
    if healthy {
        artifact_ref_if_exists(run_dir, run_id, relative_path)
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

fn coupling_status_for_required(
    _refs: &BTreeMap<String, String>,
    _required: &[&str],
) -> ContractEvidenceStatus {
    ContractEvidenceStatus::Insufficient
}

fn coupling_class_for_required(
    refs: &BTreeMap<String, String>,
    required: &[&str],
) -> ResourceCouplingEvidenceClass {
    if required.iter().all(|kind| refs.contains_key(*kind)) {
        ResourceCouplingEvidenceClass::IngredientsOnly
    } else {
        ResourceCouplingEvidenceClass::CouplingNotMeasured
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
    coupling_evidence_class: ResourceCouplingEvidenceClass,
    evidence_refs: Vec<String>,
    next_evidence_needed: Vec<ContractEvidenceGap>,
) -> ResourceCouplingChain {
    ResourceCouplingChain {
        chain_id: chain_id.to_string(),
        pressure: pressure.to_string(),
        platform_response: platform_response.to_string(),
        secondary_pressure: secondary_pressure.to_string(),
        performance_degradation: performance_degradation.to_string(),
        recovery_behavior: recovery_behavior.to_string(),
        status,
        coupling_evidence_class,
        evidence_refs,
        next_evidence_needed,
    }
}

#[allow(clippy::too_many_arguments)]
fn operating_rule(
    rule_id: &str,
    category: OperatingRuleCategory,
    statement: &str,
    rule_source: OperatingRuleSource,
    derivation: &str,
    evidence_refs: Vec<String>,
    confidence: ContractConfidence,
    allowed_design: &[&str],
    blocked_design: &[&str],
) -> OperatingContractRule {
    OperatingContractRule {
        rule_id: rule_id.to_string(),
        category,
        statement: statement.to_string(),
        rule_source,
        derivation: derivation.to_string(),
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
    next_evidence_needed: Vec<ContractEvidenceGap>,
) -> OperatingBoundary {
    OperatingBoundary {
        boundary_id: boundary_id.to_string(),
        statement: statement.to_string(),
        status,
        evidence_refs,
        next_evidence_needed,
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

    #[test]
    fn platform_inventory_does_not_treat_cpufreq_surface_as_control_evidence() {
        let temp = tempfile::tempdir().unwrap();
        write_run_context(temp.path(), "LAB-RUN-cpufreq-surface-only");
        write_cpufreq_surface_inventory(temp.path());

        let inventory = platform_mechanism_inventory_for_run(
            temp.path(),
            "pi4-target55".to_string(),
            "raspberry_pi_4".to_string(),
        )
        .unwrap();
        let cpufreq = inventory
            .mechanisms
            .iter()
            .find(|mechanism| mechanism.mechanism_id == "cpu.cpufreq_governor")
            .unwrap();

        assert_eq!(
            cpufreq.visibility_status,
            ContractEvidenceStatus::MeasuredPartial
        );
        assert_eq!(
            cpufreq.platform_control_status,
            ContractEvidenceStatus::Insufficient
        );
        assert!(cpufreq
            .reason
            .contains("no approved apply/verify/restore control result"));
    }

    #[test]
    fn platform_inventory_accepts_complete_cpufreq_control_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let run_id = "LAB-RUN-cpufreq-complete-control";
        write_run_context(temp.path(), run_id);
        write_cpufreq_surface_inventory(temp.path());

        let run = crate::run::RunContext {
            run_id: run_id.to_string(),
            run_dir: temp.path().to_path_buf(),
        };
        let target = crate::TargetSpec::parse("local").unwrap();
        let plan = crate::control::new_cpufreq_plan(
            &run,
            &target,
            "performance".to_string(),
            60,
            Some(75.0),
        );
        let approval = crate::control::new_approval_record(
            &plan,
            "operator".to_string(),
            "Set CPU governor for bounded contract evidence".to_string(),
        )
        .unwrap();
        let lease = RestoreLease {
            schema_version: "lab.restore_lease.v1".to_string(),
            lease_id: "LEASE-cpufreq-complete".to_string(),
            target_id: plan.target_id.clone(),
            operation_id: plan.operation.operation_id.clone(),
            captured_state: crate::contracts::CapturedState {
                cpufreq_policies: vec![crate::contracts::CpufreqPolicyState {
                    policy: "policy0".to_string(),
                    governor: "ondemand".to_string(),
                    scaling_min_freq: Some("600000".to_string()),
                    scaling_max_freq: Some("1800000".to_string()),
                }],
            },
            applied_state: crate::contracts::AppliedState {
                governor: "performance".to_string(),
            },
            restore_required: true,
            restore_status: RestoreStatus::Pending,
            created_by_plan: plan.plan_id.clone(),
            time_unix_ms: 1780000000000,
        };
        let applied_result = ControlResult {
            schema_version: "lab.control_result.v1".to_string(),
            result_id: "RESULT-cpufreq-applied".to_string(),
            plan_id: plan.plan_id.clone(),
            target_id: plan.target_id.clone(),
            operation_id: plan.operation.operation_id.clone(),
            risk_tier: plan.risk_tier.clone(),
            status: ControlResultStatus::Applied,
            refusal: None,
            restore_lease: Some(lease.clone()),
            restore_attempted: false,
            restore_result: None,
            time_unix_ms: 1780000000001,
        };
        let restored_lease = RestoreLease {
            restore_status: RestoreStatus::Restored,
            ..lease
        };
        let restored_result = ControlResult {
            schema_version: "lab.control_result.v1".to_string(),
            result_id: "RESULT-cpufreq-restored".to_string(),
            plan_id: plan.plan_id.clone(),
            target_id: plan.target_id.clone(),
            operation_id: plan.operation.operation_id.clone(),
            risk_tier: plan.risk_tier.clone(),
            status: ControlResultStatus::Restored,
            refusal: None,
            restore_lease: Some(restored_lease.clone()),
            restore_attempted: true,
            restore_result: Some(crate::contracts::RestoreAttempt {
                status: RestoreAttemptStatus::Succeeded,
                message: "restore command completed and verified".to_string(),
            }),
            time_unix_ms: 1780000000002,
        };

        write_json(
            temp.path().join(format!("plans/{}.json", plan.plan_id)),
            &plan,
        );
        write_json(
            temp.path()
                .join(format!("approvals/{}.json", approval.approval_id)),
            &approval,
        );
        write_json(
            temp.path()
                .join(format!("plans/{}.result.json", applied_result.result_id)),
            &applied_result,
        );
        write_json(
            temp.path()
                .join(format!("leases/{}.json", restored_lease.lease_id)),
            &restored_lease,
        );
        write_json(
            temp.path()
                .join(format!("plans/{}.result.json", restored_result.result_id)),
            &restored_result,
        );
        write_json(
            temp.path().join("health/restore_health_check.json"),
            &serde_json::json!({
                "schema_version": "lab.health_check.v1",
                "target_id": "local-target",
                "status": "ok",
                "inventory_available": true,
                "toolchain_available": true
            }),
        );

        let inventory = platform_mechanism_inventory_for_run(
            temp.path(),
            "local-target".to_string(),
            "raspberry_pi_4".to_string(),
        )
        .unwrap();
        let cpufreq = inventory
            .mechanisms
            .iter()
            .find(|mechanism| mechanism.mechanism_id == "cpu.cpufreq_governor")
            .unwrap();

        assert_eq!(
            cpufreq.platform_control_status,
            ContractEvidenceStatus::MeasuredPartial
        );
        assert!(cpufreq.reason.contains("approved apply/verify/restore"));
        for expected in [
            "/plans/",
            "/approvals/",
            "/leases/",
            "/health/restore_health_check.json",
        ] {
            assert!(
                cpufreq
                    .evidence_refs
                    .iter()
                    .any(|reference| reference.contains(expected)),
                "missing evidence ref containing {expected}"
            );
        }
    }

    fn write_run_context(run_dir: &Path, run_id: &str) {
        write_json(
            run_dir.join("run_context.json"),
            &serde_json::json!({
                "schema_version": "lab.run_context.v1",
                "run_id": run_id
            }),
        );
    }

    fn write_cpufreq_surface_inventory(run_dir: &Path) {
        write_json(
            run_dir.join("inventory/target_inventory.json"),
            &serde_json::json!({
                "schema_version": "lab.target_inventory.v1",
                "target_id": "pi4-target55",
                "target": "ssh://target55",
                "collected_by": "adc-lab",
                "time_unix_ms": 1780000000000_u64,
                "software_stack": {
                    "os": "linux",
                    "kernel": "6.x",
                    "arch": "aarch64",
                    "board": "raspberry_pi_4"
                },
                "hardware": {
                    "cpu_count": 4,
                    "memory_total_kb": 8388608,
                    "thermal_zones": 1,
                    "cpufreq_policies": 1
                },
                "control_surfaces": [{
                    "surface_id": "linux.cpufreq.sysfs",
                    "available": true,
                    "requires_privilege": true
                }]
            }),
        );
    }

    fn write_json(path: impl AsRef<Path>, value: &impl serde::Serialize) {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    }
}
