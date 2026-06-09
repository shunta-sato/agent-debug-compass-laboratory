# Demo: target55 Short Smoke

This directory contains a demo evidence pack produced from a real `target55` run on 2026-06-08.

It is intentionally under `examples/demos/` rather than canonical `docs/` because it is target-specific evidence, not product documentation or a general Raspberry Pi claim.

## Contents

- `target_profile.yaml`: demo target profile for `ssh://target55`.
- `docs/`: human-readable characterization, operating-envelope, and familiarization summaries.
- `reports/`: normalized JSON summaries derived from ignored `lab/runs/LAB-RUN-target55-*` artifacts.
- `baselines/resource/`: short-smoke baseline summaries.

## Claim Boundary

Allowed:

- `target55` completed the captured short smoke scenarios.
- The demo shows what an adc-lab familiarization pack can look like.

Blocked:

- Production physical-footprint claims.
- Sustained thermal safety claims.
- Battery, flash-wear, wakeup, latency, or jitter claims.
- Controlled cpufreq sweep claims.

Raw `lab/runs/` artifacts are intentionally not tracked in the repository.
