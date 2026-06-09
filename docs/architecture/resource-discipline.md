# Resource Discipline

Embedded-facing features provide:

- NFR matrix
- physical budget file
- target profile
- resource harness plan
- hot-path review when loops or sampling are present
- observer-effect review when measurement can perturb the target
- gate report before final submit readiness

Default blockers:

- unsupported low-overhead, battery-safe, or production-ready claim
- missing sampling cadence budget for target-local default behavior
- measurement unknown treated as pass
- host fallback treated as target proof
- continuous default storage write with no flash-wear estimate
- battery unknown treated as AC power
