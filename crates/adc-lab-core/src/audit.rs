use crate::contracts::{Actor, AuditEvent, RiskTier, POLICY_VERSION};
use crate::fsutil::append_json_line;
use crate::ids::{new_id, now_unix_ms};
use crate::{LabResult, RunContext};

pub struct AuditInput {
    pub target_id: String,
    pub actor: Actor,
    pub operation: String,
    pub operation_id: Option<String>,
    pub risk_tier: RiskTier,
    pub approval_ref: Option<String>,
    pub restore_lease_ref: Option<String>,
    pub result: String,
}

pub fn append_audit_event(run: &RunContext, input: AuditInput) -> LabResult<AuditEvent> {
    let event = AuditEvent {
        schema_version: "lab.audit_event.v1".to_string(),
        event_id: new_id("EVT"),
        run_id: run.run_id.clone(),
        target_id: input.target_id,
        actor: input.actor,
        operation: input.operation,
        operation_id: input.operation_id,
        risk_tier: input.risk_tier,
        approval_ref: input.approval_ref,
        restore_lease_ref: input.restore_lease_ref,
        result: input.result,
        policy_version: POLICY_VERSION.to_string(),
        time_unix_ms: now_unix_ms(),
    };
    append_json_line(run.run_dir.join("audit.jsonl"), &event)?;
    Ok(event)
}
