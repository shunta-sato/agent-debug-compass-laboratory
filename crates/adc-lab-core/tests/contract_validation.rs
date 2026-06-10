use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn contract_validation_schema_fixtures_validate() {
    let root = workspace_root();
    let schema_dir = root.join("schemas");
    let golden_dir = root.join("tests/golden");
    let mut checked = 0usize;

    for entry in fs::read_dir(&schema_dir).unwrap() {
        let schema_path = entry.unwrap().path();
        if schema_path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let file_name = schema_path.file_name().unwrap().to_str().unwrap();
        let fixture_name = file_name.replace(".schema.json", ".valid.json");
        let fixture_path = golden_dir.join(fixture_name);
        assert!(
            fixture_path.exists(),
            "missing golden fixture for {}",
            schema_path.display()
        );

        let schema_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&schema_path).unwrap()).unwrap();
        let fixture_json: serde_json::Value =
            serde_json::from_slice(&fs::read(&fixture_path).unwrap()).unwrap();
        validate_schema(&schema_json, &schema_json, &fixture_json, "$")
            .unwrap_or_else(|err| panic!("{}: {err}", fixture_path.display()));
        checked += 1;
    }

    assert!(checked >= 24, "expected all MVP schemas to be checked");
}

#[test]
fn contract_validation_control_plan_rejects_arbitrary_shell_field() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.control_plan.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.control_plan.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json.as_object_mut().unwrap().insert(
        "shell".to_string(),
        serde_json::json!("sudo sh -c echo bad"),
    );
    assert!(validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err());
}

#[test]
fn contract_validation_experiment_run_accepts_not_implemented_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.experiment_run.v1.schema.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let instance = serde_json::json!({
        "schema_version": "lab.experiment_run.v1",
        "run_id": "LAB-RUN-001",
        "matrix_id": "MATRIX-001",
        "target_id": "pi4-target55",
        "dry_run": false,
        "trials": [
            {
                "trial_id": "TRIAL-001",
                "factors": {
                    "governor": "performance"
                },
                "status": "not_implemented",
                "artifact_refs": [],
                "failure": "experiment execution is not implemented",
                "started_at_unix_ms": null,
                "ended_at_unix_ms": null
            }
        ],
        "time_unix_ms": 1780000000000u64
    });
    validate_schema(&schema_json, &schema_json, &instance, "$")
        .unwrap_or_else(|err| panic!("not_implemented experiment run should validate: {err}"));
}

#[test]
fn contract_validation_operating_point_coverage_rejects_legacy_provisional_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.operating_point_coverage.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.operating_point_coverage.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["coverage_status"] = serde_json::json!("provisional");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "operating point coverage must use explicit PR7 coverage statuses"
    );
}

#[test]
fn contract_validation_capability_cost_model_rejects_legacy_string_capabilities() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.capability_cost_model.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.capability_cost_model.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["capabilities"] = serde_json::json!(["cpu", "gpu"]);
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "capability cost model must use structured capability evidence entries"
    );
}

#[test]
fn contract_validation_tool_qualification_requires_agent_adapter_evidence_fields() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.tool_qualification.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.tool_qualification.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json
        .as_object_mut()
        .unwrap()
        .remove("qualification_scope");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "tool qualification must expose qualification scope"
    );
}

#[test]
fn contract_validation_privilege_provider_rejects_unknown_availability() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.privilege_provider_status.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.privilege_provider_status.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["providers"][1]["availability"] = serde_json::json!("enabled_by_default");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "privilege provider availability must stay in the schema enum"
    );
}

#[test]
fn contract_validation_workload_profile_rejects_unknown_claim_boundary() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.workload_profile.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.workload_profile.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["claim_boundary"] = serde_json::json!("selection_decision");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "workload profile claim boundary must stay in the schema enum"
    );
}

#[test]
fn contract_validation_target_capability_profile_rejects_unknown_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.target_capability_profile.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.target_capability_profile.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["capability_status"] = serde_json::json!("selection_ready");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "target capability profile status must not smuggle selection decisions"
    );
}

#[test]
fn contract_validation_target_capability_profile_requires_selection_ready_field() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.target_capability_profile.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.target_capability_profile.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json
        .as_object_mut()
        .unwrap()
        .remove("selection_ready");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "target capability profile must expose selection readiness explicitly"
    );
}

#[test]
fn contract_validation_resource_pressure_rejects_unsupported_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.resource_pressure_result.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.resource_pressure_result.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["status"] = serde_json::json!("unsupported_by_adc_lab");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "resource pressure results must classify surfaces explicitly instead of using unsupported_by_adc_lab"
    );
}

#[test]
fn contract_validation_platform_inventory_rejects_legacy_control_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.platform_mechanism_inventory.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.platform_mechanism_inventory.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    let mechanism = fixture_json["mechanisms"][0].as_object_mut().unwrap();
    mechanism.insert(
        "control_status".to_string(),
        serde_json::json!("measured_partial"),
    );
    mechanism.remove("platform_control_status");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "platform mechanism inventory must split platform control from pressure probe status"
    );
}

#[test]
fn contract_validation_resource_pressure_requires_evidence_class() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.resource_pressure_result.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.resource_pressure_result.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json
        .as_object_mut()
        .unwrap()
        .remove("evidence_class");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "resource pressure results must state smoke vs pressure-induced evidence class"
    );
}

#[test]
fn contract_validation_coupling_requires_evidence_class() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.resource_coupling_report.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.resource_coupling_report.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["chains"][0]
        .as_object_mut()
        .unwrap()
        .remove("coupling_evidence_class");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "resource coupling chains must distinguish ingredients-only evidence from composite measurement"
    );
}

#[test]
fn contract_validation_operating_rule_requires_rule_source() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.target_operating_contract.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.target_operating_contract.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["rules"][0]
        .as_object_mut()
        .unwrap()
        .remove("rule_source");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "target operating contract rules must state generic/evidence-needed/measured source"
    );
}

#[test]
fn contract_validation_operating_contract_rejects_unsupported_boundary_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.target_operating_contract.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.target_operating_contract.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["boundaries"][0]["status"] = serde_json::json!("unsupported_by_adc_lab");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "target operating contract boundaries must not use unsupported_by_adc_lab"
    );
}

#[test]
fn contract_validation_run_set_rejects_unsupported_pack_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.run_set_manifest.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.run_set_manifest.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["pack_status"] = serde_json::json!("unsupported_by_adc_lab");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "run-set pack status must use explicit operating-contract states"
    );
}

#[test]
fn contract_validation_privilege_doctor_rejects_unknown_status() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.privilege_doctor.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.privilege_doctor.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["status"] = serde_json::json!("password_prompt_waiting");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "privilege doctor must classify non-interactive readiness explicitly"
    );
}

#[test]
fn contract_validation_release_manifest_rejects_missing_binary_sha() {
    let root = workspace_root();
    let schema_path = root.join("schemas/lab.release_manifest.v1.schema.json");
    let fixture_path = root.join("tests/golden/lab.release_manifest.v1.valid.json");
    let schema_json: serde_json::Value =
        serde_json::from_slice(&fs::read(schema_path).unwrap()).unwrap();
    let mut fixture_json: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture_path).unwrap()).unwrap();
    fixture_json["binaries"][0]
        .as_object_mut()
        .unwrap()
        .remove("sha256");
    assert!(
        validate_schema(&schema_json, &schema_json, &fixture_json, "$").is_err(),
        "release manifest must record binary checksums"
    );
}

fn validate_schema(
    root: &serde_json::Value,
    schema: &serde_json::Value,
    instance: &serde_json::Value,
    path: &str,
) -> Result<(), String> {
    if let Some(reference) = schema.get("$ref").and_then(|value| value.as_str()) {
        if let Some(name) = reference.strip_prefix("#/$defs/") {
            let target = root
                .get("$defs")
                .and_then(|defs| defs.get(name))
                .ok_or_else(|| format!("{path}: unresolved ref {reference}"))?;
            return validate_schema(root, target, instance, path);
        }
        return Err(format!(
            "{path}: external ref {reference} is not supported by strict minimal validator"
        ));
    }

    if let Some(any_of) = schema.get("anyOf").and_then(|value| value.as_array()) {
        if any_of
            .iter()
            .any(|candidate| validate_schema(root, candidate, instance, path).is_ok())
        {
            return Ok(());
        }
        return Err(format!("{path}: did not match anyOf"));
    }

    if let Some(expected) = schema.get("const") {
        if instance != expected {
            return Err(format!("{path}: expected const {expected}, got {instance}"));
        }
    }

    if let Some(values) = schema.get("enum").and_then(|value| value.as_array()) {
        if !values.iter().any(|value| value == instance) {
            return Err(format!("{path}: value {instance} not in enum"));
        }
    }

    if let Some(type_spec) = schema.get("type") {
        validate_type(type_spec, instance, path)?;
    }

    if let Some(minimum) = schema.get("minimum").and_then(|value| value.as_f64()) {
        let value = instance
            .as_f64()
            .ok_or_else(|| format!("{path}: minimum requires numeric instance"))?;
        if value < minimum {
            return Err(format!("{path}: value {value} below minimum {minimum}"));
        }
    }
    if let Some(maximum) = schema.get("maximum").and_then(|value| value.as_f64()) {
        let value = instance
            .as_f64()
            .ok_or_else(|| format!("{path}: maximum requires numeric instance"))?;
        if value > maximum {
            return Err(format!("{path}: value {value} above maximum {maximum}"));
        }
    }

    if let Some(min_length) = schema.get("minLength").and_then(|value| value.as_u64()) {
        let value = instance
            .as_str()
            .ok_or_else(|| format!("{path}: minLength requires string instance"))?;
        if value.len() < min_length as usize {
            return Err(format!("{path}: string too short"));
        }
    }

    if let Some(min_items) = schema.get("minItems").and_then(|value| value.as_u64()) {
        let value = instance
            .as_array()
            .ok_or_else(|| format!("{path}: minItems requires array instance"))?;
        if value.len() < min_items as usize {
            return Err(format!("{path}: array too short"));
        }
    }

    if let Some(required) = schema.get("required").and_then(|value| value.as_array()) {
        let object = instance
            .as_object()
            .ok_or_else(|| format!("{path}: required requires object instance"))?;
        for key in required.iter().filter_map(|value| value.as_str()) {
            if !object.contains_key(key) {
                return Err(format!("{path}: missing required key {key}"));
            }
        }
    }

    if let Some(properties) = schema.get("properties").and_then(|value| value.as_object()) {
        if let Some(object) = instance.as_object() {
            for (key, property_schema) in properties {
                if let Some(value) = object.get(key) {
                    validate_schema(root, property_schema, value, &format!("{path}.{key}"))?;
                }
            }

            if schema.get("additionalProperties") == Some(&serde_json::Value::Bool(false)) {
                for key in object.keys() {
                    if !properties.contains_key(key) {
                        return Err(format!("{path}: additional property {key}"));
                    }
                }
            } else if let Some(extra_schema) = schema.get("additionalProperties") {
                if extra_schema.is_object() {
                    for (key, value) in object {
                        if !properties.contains_key(key) {
                            validate_schema(root, extra_schema, value, &format!("{path}.{key}"))?;
                        }
                    }
                }
            }
        }
    }

    if let Some(items) = schema.get("items") {
        if let Some(array) = instance.as_array() {
            for (index, value) in array.iter().enumerate() {
                validate_schema(root, items, value, &format!("{path}[{index}]"))?;
            }
        }
    }

    Ok(())
}

#[test]
fn contract_validation_ci_workflow_runs_make_verify_with_read_only_contents() {
    let root = workspace_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/ci.yml")).unwrap();
    assert_workflow_yaml_parses(&workflow);
    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("branches:"));
    assert!(workflow.contains("- main"));
    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("permissions:\n  contents: read"));
    assert!(workflow.contains("run: make verify"));
}

#[test]
fn contract_validation_release_workflow_publishes_checksummed_assets_with_scoped_permissions() {
    let root = workspace_root();
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    assert_workflow_yaml_parses(&workflow);
    assert!(workflow.contains("tags:"));
    assert!(workflow.contains("'v*'"));
    assert!(workflow.contains("aarch64-unknown-linux-gnu"));
    assert!(workflow.contains("x86_64-unknown-linux-gnu"));
    assert!(workflow.contains("contents: write"));
    assert!(workflow.contains("id-token: write"));
    assert!(workflow.contains("attestations: write"));
    assert!(workflow.contains("sha256sum -c SHA256SUMS"));
    assert!(workflow.contains("install-adc-lab-helper.sh"));
    assert!(workflow.contains("sha256sum *.tar.gz install-adc-lab-helper.sh > SHA256SUMS"));
    assert!(workflow.contains(
        "assets=(dist-assets/*.tar.gz dist-assets/install-adc-lab-helper.sh dist-assets/SHA256SUMS)"
    ));
    assert!(workflow.contains("ADC_LAB_VERSION=\"${{ steps.release.outputs.version }}\""));
    assert!(workflow.contains("RELEASE_TAG_INPUT: ${{ inputs.tag }}"));
    assert!(workflow.contains("tag=\"$RELEASE_TAG_INPUT\""));
    assert!(workflow.contains("RELEASE_TAG: ${{ steps.release.outputs.tag }}"));
    assert!(workflow.contains("tag=\"$RELEASE_TAG\""));
    assert!(!workflow.contains("tag=\"${{ inputs.tag }}\""));
    assert!(
        workflow_run_scripts(&workflow)
            .iter()
            .all(|script| !script.contains("${{ inputs.")),
        "workflow_dispatch inputs must be passed through env, not interpolated directly into run scripts"
    );
    assert!(workflow.contains("--notes-file release-notes.md"));
    assert!(workflow.contains("gh release create"));
    assert!(workflow.contains("actions/upload-artifact@v4"));
    assert!(workflow.contains("actions/download-artifact@v4"));
    assert!(workflow.contains("actions/attest-build-provenance@v2"));
}

#[test]
fn contract_validation_release_helper_installer_keeps_fixed_safety_boundary() {
    let root = workspace_root();
    let script = fs::read_to_string(root.join("scripts/install-adc-lab-helper.sh")).unwrap();
    assert!(script.starts_with("#!/usr/bin/env bash"));
    assert!(script.contains("set -euo pipefail"));
    assert!(script.contains("REPOSITORY=\"shunta-sato/agent-debug-compass-laboratory\""));
    assert!(script.contains("HELPER_DEST=\"/usr/local/libexec/adc-lab-priv-helper\""));
    assert!(script.contains("SUDOERS_DEST=\"/etc/sudoers.d/adc-lab\""));
    assert!(script.contains("do not run this installer as root"));
    assert!(script.contains("validate_version \"$version\""));
    assert!(script.contains("validate_sudo_user \"$sudo_user\" \"$current_user\""));
    assert!(script.contains("sha256sum -c SHA256SUMS --ignore-missing"));
    assert!(script.contains("sudo install -o root -g root -m 0755"));
    assert!(script.contains("sudo visudo -cf \"$sudoers_tmp\""));
    assert!(script.contains("sudo -n \"$HELPER_DEST\" --version"));
    assert!(script.contains("privilege doctor"));
    assert!(!script.contains("eval "));
    assert!(!script.contains("| sudo"));
    assert!(!script.contains("sudo bash"));
    assert!(!script.contains("sudo sh"));
}

fn assert_workflow_yaml_parses(workflow: &str) {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(workflow).expect("workflow yaml must parse");
    assert!(parsed.is_mapping(), "workflow yaml must be a mapping");
}

fn workflow_run_scripts(workflow: &str) -> Vec<String> {
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(workflow).expect("workflow yaml must parse");
    let mut scripts = Vec::new();
    let Some(jobs) = yaml_mapping_get(&parsed, "jobs").and_then(|value| value.as_mapping()) else {
        return scripts;
    };
    for job in jobs.values() {
        let Some(steps) = yaml_mapping_get(job, "steps").and_then(|value| value.as_sequence())
        else {
            continue;
        };
        for step in steps {
            if let Some(script) = yaml_mapping_get(step, "run").and_then(|value| value.as_str()) {
                scripts.push(script.to_string());
            }
        }
    }
    scripts
}

fn yaml_mapping_get<'a>(value: &'a serde_yaml::Value, key: &str) -> Option<&'a serde_yaml::Value> {
    value
        .as_mapping()?
        .get(serde_yaml::Value::String(key.to_string()))
}

#[test]
fn contract_validation_external_ref_is_not_silently_accepted() {
    let schema = serde_json::json!({ "$ref": "external.schema.json" });
    let instance = serde_json::json!({ "anything": true });
    let error = validate_schema(&schema, &schema, &instance, "$").unwrap_err();
    assert!(error.contains("external ref"));
}

fn validate_type(
    type_spec: &serde_json::Value,
    instance: &serde_json::Value,
    path: &str,
) -> Result<(), String> {
    if let Some(types) = type_spec.as_array() {
        if types
            .iter()
            .any(|candidate| validate_type(candidate, instance, path).is_ok())
        {
            return Ok(());
        }
        return Err(format!("{path}: type mismatch"));
    }
    let expected = type_spec
        .as_str()
        .ok_or_else(|| format!("{path}: invalid type schema"))?;
    let matches = match expected {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "number" => instance.is_number(),
        other => return Err(format!("{path}: unsupported type {other}")),
    };
    if matches {
        Ok(())
    } else {
        Err(format!("{path}: expected type {expected}, got {instance}"))
    }
}
