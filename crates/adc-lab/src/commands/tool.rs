use super::super::*;
use super::common::*;
use anyhow::Context;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct PendingToolQualificationEvidence {
    evidence: ToolQualificationEvidence,
    output_schema_path: PathBuf,
    output_schema: serde_json::Value,
    dry_run_path: PathBuf,
    dry_run: serde_json::Value,
    manual_comparison_path: PathBuf,
    manual_comparison: serde_json::Value,
    static_safety_review_path: PathBuf,
    static_safety_review: String,
}

pub(crate) fn command_tool_qualify(args: ToolQualifyCommand) -> Result<()> {
    let run = create_or_open_run(args.run_dir.clone())?;
    let manifest: ToolManifest = read_yaml(&args.manifest)?;
    let pending_evidence = build_pending_tool_qualification_evidence(&run, &manifest, &args)?;
    let report = qualify_tool_with_evidence(
        manifest,
        pending_evidence
            .as_ref()
            .map(|pending| pending.evidence.clone()),
    )?;
    if let Some(pending) = pending_evidence.as_ref() {
        persist_pending_tool_qualification_evidence(&run, pending)?;
    }
    let path = run
        .run_dir
        .join("tools")
        .join(format!("{}.qualification.json", report.tool_id));
    write_json_artifact(&run, &path, &report)?;
    append_audit_event(
        &run,
        AuditInput {
            target_id: "toolchain".to_string(),
            actor: Actor::codex(),
            operation: "tool.qualify".to_string(),
            operation_id: Some(report.tool_id.clone()),
            risk_tier: RiskTier::Tier0ReadOnlyObservation,
            approval_ref: None,
            restore_lease_ref: None,
            result: if report.evidence_accepted {
                "qualified".to_string()
            } else {
                "recorded_unqualified".to_string()
            },
        },
    )?;
    print_artifact(&run, &path, report)
}

fn build_pending_tool_qualification_evidence(
    run: &RunContext,
    manifest: &ToolManifest,
    args: &ToolQualifyCommand,
) -> Result<Option<PendingToolQualificationEvidence>> {
    if !tool_qualification_evidence_requested(args) {
        return Ok(None);
    }

    let Some(tool_version) = args.tool_version.clone() else {
        anyhow::bail!("complete tool qualification evidence requires --tool-version");
    };
    let Some(tool_sha256) = args.tool_sha256.clone() else {
        anyhow::bail!("complete tool qualification evidence requires --tool-sha256");
    };
    let Some(output_schema_input) = args.output_schema.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --output-schema");
    };
    let Some(dry_run_input) = args.dry_run_output.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --dry-run-output");
    };
    let Some(manual_comparison_input) = args.manual_comparison.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --manual-comparison");
    };
    let Some(static_safety_review_input) = args.static_safety_review.as_ref() else {
        anyhow::bail!("complete tool qualification evidence requires --static-safety-review");
    };

    let output_schema = read_json_evidence_file(
        output_schema_input,
        "output schema",
        AGENT_ADAPTER_OUTPUT_BYTES_MAX,
    )?
    .0;
    let (dry_run, validated_output_bytes) = read_json_evidence_file(
        dry_run_input,
        "dry-run output",
        manifest.bounded.output_bytes_max,
    )?;
    let manual_comparison = read_json_evidence_file(
        manual_comparison_input,
        "manual comparison",
        manifest.bounded.output_bytes_max,
    )?
    .0;
    let static_safety_review = read_text_evidence_file(
        static_safety_review_input,
        "static safety review",
        64 * 1024,
    )?;

    let safe_tool_id = safe_artifact_id(&manifest.tool_id, "TOOL");
    let output_schema_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.output_schema.json"));
    let dry_run_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.dry_run.json"));
    let manual_comparison_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.manual_comparison.json"));
    let static_safety_review_path = run
        .run_dir
        .join("tools")
        .join(format!("{safe_tool_id}.static_safety_review.txt"));

    let evidence = ToolQualificationEvidence {
        tool_version,
        tool_sha256,
        output_schema_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.output_schema.json"),
        ),
        dry_run_ref: planned_run_artifact_ref(run, &format!("tools/{safe_tool_id}.dry_run.json")),
        manual_comparison_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.manual_comparison.json"),
        ),
        static_safety_review_ref: planned_run_artifact_ref(
            run,
            &format!("tools/{safe_tool_id}.static_safety_review.txt"),
        ),
        validated_output_bytes,
    };

    Ok(Some(PendingToolQualificationEvidence {
        evidence,
        output_schema_path,
        output_schema,
        dry_run_path,
        dry_run,
        manual_comparison_path,
        manual_comparison,
        static_safety_review_path,
        static_safety_review,
    }))
}

fn tool_qualification_evidence_requested(args: &ToolQualifyCommand) -> bool {
    args.tool_version.is_some()
        || args.tool_sha256.is_some()
        || args.output_schema.is_some()
        || args.dry_run_output.is_some()
        || args.manual_comparison.is_some()
        || args.static_safety_review.is_some()
}

fn persist_pending_tool_qualification_evidence(
    run: &RunContext,
    pending: &PendingToolQualificationEvidence,
) -> Result<()> {
    write_json_artifact(run, &pending.output_schema_path, &pending.output_schema)?;
    write_json_artifact(run, &pending.dry_run_path, &pending.dry_run)?;
    write_json_artifact(
        run,
        &pending.manual_comparison_path,
        &pending.manual_comparison,
    )?;
    write_text_artifact(
        run,
        &pending.static_safety_review_path,
        &pending.static_safety_review,
    )?;
    Ok(())
}

fn write_text_artifact(run: &RunContext, path: &Path, value: &str) -> Result<String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create artifact directory {}", parent.display()))?;
    }
    fs::write(path, value)
        .with_context(|| format!("failed to write artifact {}", path.display()))?;
    Ok(run.artifact_uri(path)?)
}

fn planned_run_artifact_ref(run: &RunContext, relative_path: &str) -> String {
    format!("artifact://lab/runs/{}/{}", run.run_id, relative_path)
}

fn read_json_evidence_file(
    path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<(serde_json::Value, u64)> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {label} evidence"))?;
    let len = bytes.len() as u64;
    if len == 0 {
        anyhow::bail!("{label} evidence is empty");
    }
    if len > max_bytes {
        anyhow::bail!("{label} evidence exceeds {max_bytes} byte bound");
    }
    let value = serde_json::from_slice(&bytes)
        .with_context(|| format!("{label} evidence must be valid JSON"))?;
    Ok((value, len))
}

fn read_text_evidence_file(path: &Path, label: &str, max_bytes: u64) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {label} evidence"))?;
    if bytes.is_empty() {
        anyhow::bail!("{label} evidence is empty");
    }
    if bytes.len() as u64 > max_bytes {
        anyhow::bail!("{label} evidence exceeds {max_bytes} byte bound");
    }
    let text =
        String::from_utf8(bytes).with_context(|| format!("{label} evidence must be UTF-8 text"))?;
    if text.trim().is_empty() {
        anyhow::bail!("{label} evidence is blank");
    }
    Ok(text)
}

pub(crate) fn command_tool_qualify_inventory(args: ToolQualifyInventoryCommand) -> Result<()> {
    let run = create_or_open_run(
        args.run_dir
            .or_else(|| infer_run_dir_from_artifact(&args.inventory)),
    )?;
    let inventory: ToolchainInventory = read_json(&args.inventory)?;
    let inventory_ref = run.artifact_uri(&args.inventory).ok();
    let (summary, path, _) = persist_toolchain_qualifications(&run, &inventory, inventory_ref)?;
    print_artifact(&run, &path, summary)
}

fn infer_run_dir_from_artifact(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    match parent.file_name().and_then(|name| name.to_str()) {
        Some("inventory" | "toolchain" | "observations" | "reports" | "tools") => {
            parent.parent().map(Path::to_path_buf)
        }
        _ => None,
    }
}
