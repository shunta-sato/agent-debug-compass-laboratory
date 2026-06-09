use crate::BuildInfo;

pub fn build_info(name: &str) -> BuildInfo {
    BuildInfo {
        name: name.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: env!("ADC_LAB_GIT_SHA").to_string(),
        target_triple: env!("ADC_LAB_TARGET_TRIPLE").to_string(),
        build_profile: env!("ADC_LAB_BUILD_PROFILE").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::build_info;

    #[test]
    fn build_info_has_required_fields() {
        let value = build_info("adc-lab");
        assert_eq!(value.name, "adc-lab");
        assert!(!value.version.is_empty());
        assert!(!value.git_sha.is_empty());
        assert!(!value.target_triple.is_empty());
        assert!(!value.build_profile.is_empty());
    }
}
