use crate::{LabError, LabResult};
use std::path::Path;

pub const LOCAL_TARGET_ID: &str = "local-target";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetTransport {
    Local,
    Ssh,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetSpec {
    pub raw: String,
    pub transport: TargetTransport,
    pub endpoint: String,
    pub target_id: String,
}

impl TargetSpec {
    pub fn parse(raw: &str) -> LabResult<Self> {
        if raw == "local" || raw == "local://" {
            return Ok(Self {
                raw: raw.to_string(),
                transport: TargetTransport::Local,
                endpoint: "local".to_string(),
                target_id: LOCAL_TARGET_ID.to_string(),
            });
        }

        if let Some(endpoint) = raw.strip_prefix("ssh://") {
            if endpoint.is_empty() || !is_safe_ssh_endpoint(endpoint) {
                return Err(LabError::InvalidTarget(raw.to_string()));
            }
            let target_id = endpoint
                .chars()
                .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
                .collect::<String>();
            return Ok(Self {
                raw: raw.to_string(),
                transport: TargetTransport::Ssh,
                endpoint: endpoint.to_string(),
                target_id,
            });
        }

        Err(LabError::InvalidTarget(raw.to_string()))
    }
}

fn is_safe_ssh_endpoint(endpoint: &str) -> bool {
    !endpoint.starts_with('-')
        && endpoint
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | '@'))
}

pub fn ssh_runner_program() -> LabResult<String> {
    match std::env::var("ADC_LAB_TARGET_RUNNER") {
        Ok(value) => validate_ssh_runner_program(&value).map(|()| value),
        Err(_) => Ok("adc-lab-target".to_string()),
    }
}

pub fn validate_ssh_runner_program(value: &str) -> LabResult<()> {
    if value == "adc-lab-target" {
        return Ok(());
    }
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-')))
    {
        return Err(LabError::Policy(
            "ADC_LAB_TARGET_RUNNER must be a fixed adc-lab-target path, not a shell fragment"
                .to_string(),
        ));
    }
    let path = Path::new(value);
    if !path.is_absolute()
        || path.file_name().and_then(|name| name.to_str()) != Some("adc-lab-target")
    {
        return Err(LabError::Policy(
            "ADC_LAB_TARGET_RUNNER must be adc-lab-target or an allowed absolute adc-lab-target path"
                .to_string(),
        ));
    }
    let allowed_system_path =
        value == "/usr/local/bin/adc-lab-target" || value == "/usr/bin/adc-lab-target";
    let allowed_user_local_path =
        value.starts_with("/home/") && value.ends_with("/.local/bin/adc-lab-target");
    if allowed_system_path || allowed_user_local_path {
        Ok(())
    } else {
        Err(LabError::Policy(
            "ADC_LAB_TARGET_RUNNER path is outside the adc-lab-target allowlist".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_validation_target_parse_ssh() {
        let target = TargetSpec::parse("ssh://pi4").unwrap();
        assert_eq!(target.transport, TargetTransport::Ssh);
        assert_eq!(target.target_id, "pi4");
    }

    #[test]
    fn contract_validation_target_parse_ssh_rejects_option_injection() {
        assert!(TargetSpec::parse("ssh://-oProxyCommand=bad").is_err());
        assert!(TargetSpec::parse("ssh://target55;sh").is_err());
        assert!(TargetSpec::parse("ssh://operator@target55").is_ok());
    }

    #[test]
    fn contract_validation_default_ssh_runner_is_fixed() {
        std::env::remove_var("ADC_LAB_TARGET_RUNNER");
        assert_eq!(ssh_runner_program().unwrap(), "adc-lab-target");
    }

    #[test]
    fn contract_validation_ssh_runner_rejects_shell_fragments() {
        assert!(validate_ssh_runner_program("sh -c adc-lab-target").is_err());
        assert!(validate_ssh_runner_program("/tmp/adc-lab-target").is_err());
        assert!(validate_ssh_runner_program("/home/demo/.local/bin/adc-lab-target").is_ok());
    }
}
