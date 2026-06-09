use crate::contracts::{ControlSurfaceSummary, HardwareInventory, SoftwareStack, TargetInventory};
use crate::fsutil::read_to_string_lossy;
use crate::ids::now_unix_ms;
use crate::target::{ssh_runner_program, TargetSpec, TargetTransport};
use crate::{LabError, LabResult};
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn collect_inventory(target: &TargetSpec) -> LabResult<TargetInventory> {
    match target.transport {
        TargetTransport::Local => collect_local_inventory(target),
        TargetTransport::Ssh => collect_ssh_inventory(target),
    }
}

pub fn collect_local_inventory(target: &TargetSpec) -> LabResult<TargetInventory> {
    let software_stack = SoftwareStack {
        os: os_release_pretty_name().unwrap_or_else(|| "unknown-linux".to_string()),
        kernel: uname_arg("-r").unwrap_or_else(|| "unknown".to_string()),
        arch: std::env::consts::ARCH.to_string(),
        board: read_to_string_lossy("/proc/device-tree/model")?
            .or_else(|| {
                read_to_string_lossy("/sys/firmware/devicetree/base/model")
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| "unknown-board".to_string()),
    };
    let hardware = HardwareInventory {
        cpu_count: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        memory_total_kb: mem_total_kb(),
        thermal_zones: count_prefix("/sys/class/thermal", "thermal_zone"),
        cpufreq_policies: count_prefix("/sys/devices/system/cpu/cpufreq", "policy"),
    };
    let control_surfaces = vec![
        ControlSurfaceSummary {
            surface_id: "linux.procfs".to_string(),
            available: Path::new("/proc").exists(),
            requires_privilege: false,
        },
        ControlSurfaceSummary {
            surface_id: "linux.thermal_zone".to_string(),
            available: Path::new("/sys/class/thermal").exists(),
            requires_privilege: false,
        },
        ControlSurfaceSummary {
            surface_id: "linux.cpufreq.sysfs".to_string(),
            available: Path::new("/sys/devices/system/cpu/cpufreq").exists(),
            requires_privilege: true,
        },
    ];

    Ok(TargetInventory {
        schema_version: "lab.target_inventory.v1".to_string(),
        target_id: target.target_id.clone(),
        target: target.raw.clone(),
        collected_by: "adc-lab".to_string(),
        time_unix_ms: now_unix_ms(),
        software_stack,
        hardware,
        control_surfaces,
    })
}

fn collect_ssh_inventory(target: &TargetSpec) -> LabResult<TargetInventory> {
    let output = Command::new("ssh")
        .arg(&target.endpoint)
        .arg(ssh_runner_program()?)
        .arg("inventory")
        .arg("--json")
        .output()?;
    if !output.status.success() {
        return Err(LabError::Command(format!(
            "ssh target runner inventory failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let mut inventory: TargetInventory = serde_json::from_slice(&output.stdout)?;
    inventory.target_id = target.target_id.clone();
    inventory.target = target.raw.clone();
    inventory.collected_by = "adc-lab-target via adc-lab".to_string();
    Ok(inventory)
}

fn uname_arg(arg: &str) -> Option<String> {
    let output = Command::new("uname").arg(arg).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn os_release_pretty_name() -> Option<String> {
    let text = fs::read_to_string("/etc/os-release").ok()?;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            return Some(value.trim_matches('"').to_string());
        }
    }
    None
}

fn mem_total_kb() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            return rest.split_whitespace().next()?.parse().ok();
        }
    }
    None
}

fn count_prefix(dir: &str, prefix: &str) -> usize {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| {
                    entry
                        .file_name()
                        .to_str()
                        .map(|name| name.starts_with(prefix))
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0)
}
