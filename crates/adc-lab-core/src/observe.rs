use crate::fsutil::read_to_string_lossy;
use crate::{LabError, LabResult};
use serde::{Deserialize, Serialize};
use std::fs;
use std::str::FromStr;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Signal {
    Cpu,
    Freq,
    Thermal,
    Memory,
}

impl FromStr for Signal {
    type Err = LabError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "cpu" => Ok(Self::Cpu),
            "freq" => Ok(Self::Freq),
            "thermal" => Ok(Self::Thermal),
            "memory" => Ok(Self::Memory),
            other => Err(LabError::Validation(format!("unknown signal {other}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationSample {
    pub sample_index: usize,
    pub cpu_total_ticks: Option<u64>,
    pub cpu_idle_ticks: Option<u64>,
    pub memory_available_kb: Option<u64>,
    pub avg_cpu_freq_khz: Option<u64>,
    pub max_temp_c: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ObservationResult {
    pub schema_version: String,
    pub target_id: String,
    pub duration_ms: u64,
    pub signals: Vec<Signal>,
    pub samples: Vec<ObservationSample>,
}

pub fn parse_duration(value: &str) -> LabResult<Duration> {
    humantime::parse_duration(value).map_err(|err| LabError::InvalidDuration(err.to_string()))
}

pub fn observe_local(
    target_id: String,
    duration: Duration,
    interval: Duration,
    signals: Vec<Signal>,
) -> LabResult<ObservationResult> {
    let started = Instant::now();
    let deadline = started + duration;
    let mut samples = Vec::new();
    let mut sample_index = 0;
    loop {
        samples.push(sample_local(sample_index, &signals)?);
        sample_index += 1;
        if Instant::now() >= deadline {
            break;
        }
        let now = Instant::now();
        let sleep_for = interval.min(deadline.saturating_duration_since(now));
        if sleep_for.is_zero() {
            break;
        }
        thread::sleep(sleep_for);
    }
    Ok(ObservationResult {
        schema_version: "lab.observation_result.v1".to_string(),
        target_id,
        duration_ms: started.elapsed().as_millis() as u64,
        signals,
        samples,
    })
}

pub fn sample_local(sample_index: usize, signals: &[Signal]) -> LabResult<ObservationSample> {
    let need = |signal: Signal| signals.iter().any(|s| *s == signal);
    let (cpu_total_ticks, cpu_idle_ticks) = if need(Signal::Cpu) {
        cpu_ticks().unwrap_or((0, 0))
    } else {
        (0, 0)
    };
    Ok(ObservationSample {
        sample_index,
        cpu_total_ticks: need(Signal::Cpu).then_some(cpu_total_ticks),
        cpu_idle_ticks: need(Signal::Cpu).then_some(cpu_idle_ticks),
        memory_available_kb: need(Signal::Memory).then(mem_available_kb).flatten(),
        avg_cpu_freq_khz: need(Signal::Freq).then(avg_cpu_freq_khz).flatten(),
        max_temp_c: need(Signal::Thermal).then(max_temp_c).flatten(),
    })
}

pub fn max_temp_c() -> Option<f64> {
    let entries = fs::read_dir("/sys/class/thermal").ok()?;
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name.starts_with("thermal_zone"))
                .unwrap_or(false)
        })
        .filter_map(|entry| {
            let raw = read_to_string_lossy(entry.path().join("temp"))
                .ok()
                .flatten()?;
            let milli_c: f64 = raw.parse().ok()?;
            Some(milli_c / 1000.0)
        })
        .fold(None, |max, value| match max {
            Some(current) if current >= value => Some(current),
            _ => Some(value),
        })
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
