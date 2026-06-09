use crate::contracts::{
    PrivilegeLevel, QualificationStatus, ToolCategory, ToolInfo, ToolchainInventory,
};
use crate::{collect_inventory, LabResult, TargetSpec, TargetTransport};
use std::path::Path;

pub fn discover_toolchain(target: &TargetSpec) -> LabResult<ToolchainInventory> {
    match target.transport {
        TargetTransport::Local => discover_local_toolchain(target),
        TargetTransport::Ssh => {
            let inventory = collect_inventory(target)?;
            Ok(ToolchainInventory {
                schema_version: "lab.toolchain_inventory.v1".to_string(),
                target_id: inventory.target_id,
                software_stack: inventory.software_stack,
                tools: vec![ToolInfo {
                    tool_id: "adc-lab-target".to_string(),
                    category: ToolCategory::Probe,
                    available: true,
                    privilege: PrivilegeLevel::None,
                    qualification: QualificationStatus::Builtin,
                }],
            })
        }
    }
}

pub fn discover_local_toolchain(target: &TargetSpec) -> LabResult<ToolchainInventory> {
    let inventory = collect_inventory(target)?;
    Ok(ToolchainInventory {
        schema_version: "lab.toolchain_inventory.v1".to_string(),
        target_id: inventory.target_id,
        software_stack: inventory.software_stack,
        tools: vec![
            ToolInfo {
                tool_id: "linux.procfs".to_string(),
                category: ToolCategory::Observation,
                available: Path::new("/proc").exists(),
                privilege: PrivilegeLevel::None,
                qualification: QualificationStatus::Builtin,
            },
            ToolInfo {
                tool_id: "linux.thermal_zone".to_string(),
                category: ToolCategory::Observation,
                available: Path::new("/sys/class/thermal").exists(),
                privilege: PrivilegeLevel::None,
                qualification: QualificationStatus::Builtin,
            },
            ToolInfo {
                tool_id: "linux.cpufreq.sysfs".to_string(),
                category: ToolCategory::ObservationControl,
                available: Path::new("/sys/devices/system/cpu/cpufreq").exists(),
                privilege: PrivilegeLevel::SudoHelper,
                qualification: QualificationStatus::NeedsControlTest,
            },
            ToolInfo {
                tool_id: "stress-ng".to_string(),
                category: ToolCategory::Load,
                available: executable_on_path("stress-ng"),
                privilege: PrivilegeLevel::User,
                qualification: QualificationStatus::ExternalUnqualified,
            },
            ToolInfo {
                tool_id: "adc-lab-builtin-cpu-load".to_string(),
                category: ToolCategory::Load,
                available: true,
                privilege: PrivilegeLevel::User,
                qualification: QualificationStatus::Builtin,
            },
        ],
    })
}

fn executable_on_path(name: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join(name);
        candidate.is_file()
    })
}
