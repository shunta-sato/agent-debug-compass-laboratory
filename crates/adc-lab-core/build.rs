use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=ADC_LAB_VERSION");
    println!("cargo:rerun-if-env-changed=ADC_LAB_GIT_SHA");

    let version = std::env::var("ADC_LAB_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| std::env::var("CARGO_PKG_VERSION").ok())
        .unwrap_or_else(|| "unknown".to_string());
    let git_sha = std::env::var("ADC_LAB_GIT_SHA")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(current_git_sha)
        .unwrap_or_else(|| "unknown".to_string());
    let target_triple = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    let build_profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=ADC_LAB_VERSION={version}");
    println!("cargo:rustc-env=ADC_LAB_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=ADC_LAB_TARGET_TRIPLE={target_triple}");
    println!("cargo:rustc-env=ADC_LAB_BUILD_PROFILE={build_profile}");
}

fn current_git_sha() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
