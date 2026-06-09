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

    assert!(checked >= 17, "expected all MVP schemas to be checked");
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
