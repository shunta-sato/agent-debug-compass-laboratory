use crate::contracts::{Actor, LoadPlan, LoadResult};
use crate::ids::{new_id, now_unix_ms};
use crate::observe::max_temp_c;
use crate::{LabError, LabResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_CPU_LOAD_DURATION_SECONDS: u64 = 300;

pub fn new_cpu_load_plan(
    target_id: String,
    workers: usize,
    duration: Duration,
    abort_temp_c: Option<f64>,
) -> LabResult<LoadPlan> {
    if workers == 0 {
        return Err(LabError::Validation("workers must be >= 1".to_string()));
    }
    if duration.is_zero() {
        return Err(LabError::Validation("duration must be > 0".to_string()));
    }
    let duration_seconds = duration.as_secs().max(1);
    if duration_seconds > MAX_CPU_LOAD_DURATION_SECONDS {
        return Err(LabError::Policy(format!(
            "cpu load duration must be <= {MAX_CPU_LOAD_DURATION_SECONDS}s by default policy"
        )));
    }
    let max_workers = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    if workers > max_workers {
        return Err(LabError::Policy(format!(
            "cpu load workers must be <= available parallelism ({max_workers}) by default policy"
        )));
    }
    Ok(LoadPlan {
        schema_version: "lab.load_plan.v1".to_string(),
        load_id: new_id("LOAD"),
        target_id,
        load_kind: "cpu".to_string(),
        workers,
        duration_seconds,
        abort_temp_c,
        created_by: Actor::codex(),
        time_unix_ms: now_unix_ms(),
    })
}

pub fn run_cpu_load(plan: &LoadPlan) -> LabResult<LoadResult> {
    if plan.load_kind != "cpu" {
        return Err(LabError::Validation(
            "only cpu load is supported".to_string(),
        ));
    }
    let stop = Arc::new(AtomicBool::new(false));
    let started = Instant::now();
    let deadline = started + Duration::from_secs(plan.duration_seconds);
    let mut handles = Vec::new();

    for _ in 0..plan.workers {
        let stop = Arc::clone(&stop);
        handles.push(thread::spawn(move || {
            let mut iterations = 0u64;
            let mut value = 0x9e37_79b9_7f4a_7c15u64;
            while !stop.load(Ordering::Relaxed) {
                for _ in 0..2048 {
                    value = value.rotate_left(13).wrapping_mul(0xbf58_476d_1ce4_e5b9);
                    iterations = iterations.wrapping_add(1);
                }
                std::hint::black_box(value);
            }
            iterations
        }));
    }

    let mut abort_reason = None;
    let mut max_temp = None;
    while Instant::now() < deadline {
        if let Some(temp) = max_temp_c() {
            max_temp = Some(max_temp.map_or(temp, |current: f64| current.max(temp)));
            if let Some(limit) = plan.abort_temp_c {
                if temp >= limit {
                    abort_reason = Some(format!("thermal_abort_at_{temp:.1}c"));
                    break;
                }
            }
        }
        thread::sleep(
            Duration::from_millis(100).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    stop.store(true, Ordering::Relaxed);

    let worker_iterations = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| LabError::Validation("load worker panicked".to_string()))
        })
        .collect::<LabResult<Vec<_>>>()?;

    Ok(LoadResult {
        schema_version: "lab.load_result.v1".to_string(),
        result_id: new_id("LOAD-RESULT"),
        load_id: plan.load_id.clone(),
        target_id: plan.target_id.clone(),
        status: if abort_reason.is_some() {
            "aborted".to_string()
        } else {
            "completed".to_string()
        },
        workers: plan.workers,
        duration_ms: started.elapsed().as_millis() as u64,
        abort_reason,
        max_observed_temp_c: max_temp,
        worker_iterations,
        time_unix_ms: now_unix_ms(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_validation_cpu_load_is_bounded() {
        let plan = new_cpu_load_plan(
            "local-target".to_string(),
            1,
            Duration::from_millis(100),
            None,
        )
        .unwrap();
        let result = run_cpu_load(&plan).unwrap();
        assert_eq!(result.status, "completed");
        assert_eq!(result.worker_iterations.len(), 1);
    }

    #[test]
    fn contract_validation_cpu_load_rejects_unbounded_duration() {
        let error = new_cpu_load_plan(
            "local-target".to_string(),
            1,
            Duration::from_secs(MAX_CPU_LOAD_DURATION_SECONDS + 1),
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("duration"));
    }

    #[test]
    fn contract_validation_cpu_load_rejects_worker_excess() {
        let workers = std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1)
            + 1;
        let error = new_cpu_load_plan(
            "local-target".to_string(),
            workers,
            Duration::from_secs(1),
            None,
        )
        .unwrap_err();
        assert!(error.to_string().contains("workers"));
    }
}
