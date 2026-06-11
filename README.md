# agent-debug-compass-laboratory

[![CI](https://github.com/shunta-sato/agent-debug-compass-laboratory/actions/workflows/ci.yml/badge.svg)](https://github.com/shunta-sato/agent-debug-compass-laboratory/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/shunta-sato/agent-debug-compass-laboratory)](https://github.com/shunta-sato/agent-debug-compass-laboratory/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

```text
      _    ____   ____      _          _
     / \  |  _ \ / ___|    | |    __ _| |__
    / _ \ | | | | |   _____| |   / _` | '_ \
   / ___ \| |_| | |__|_____| |__| (_| | |_) |
  /_/   \_\____/ \____|    |_____\__,_|_.__/

        Agent Debug Compass Laboratory
        Target capability is not a guess.
```

**`adc-lab` is a target operating contract discovery laboratory for AI agents building embedded and edge software.**

It helps an Agent understand a real device before designing software for it:

* what the hardware can actually do,
* what the OS / firmware / runtime policies change,
* where performance degrades,
* which pressure conditions couple together,
* which software patterns are safe, burst-only, degraded-mode-only, or blocked,
* and which claims are unsupported because evidence is missing.

`adc-lab` is not a benchmark scoreboard.
It is a way to turn target exploration from **vibes and shell transcripts** into **audited evidence and machine-readable design constraints**.

---

## 30-second mental model

```text
adc-lab inventory          -> what is this target?
adc-lab observe            -> what is visible under current policy?
adc-lab load / pressure    -> how does it react to bounded stress?
adc-lab control            -> what happens when an operating point is changed?
adc-lab report             -> what claims are supported or blocked?
adc-lab workload run       -> what demand does a bounded local workload create?
adc-lab decide suitability -> does run-backed evidence meet policy margins?
adc-lab constraints        -> what must implementation agents obey?
```

Every step is meant to preserve evidence, record claim boundaries, and avoid pretending that a measurement proves more than it actually proves.

---

## Why this exists

AI agents are getting good at writing software.

They can write code faster than they can understand the device it will run on.

On embedded and edge devices, software performance is shaped by things like:

* CPU governors and frequency policy,
* thermal behavior and throttling,
* memory pressure and reclaim behavior,
* page cache and storage latency,
* network bursts and retry behavior,
* scheduler jitter,
* observer / logger overhead,
* target-specific control surfaces,
* and whether the “measurement tool” itself becomes the workload.

A human engineer may know that “Pi4 is probably enough” or “this target will throttle under sustained load.”
An Agent should not guess that. It should measure, preserve evidence, and stay honest about what is still unknown.

`adc-lab` exists to make that possible.

---

## What `adc-lab` does

`adc-lab` discovers a **Target Operating Contract**.

A Target Operating Contract answers questions like:

```text
What can this target do?
What platform mechanisms affect software performance?
Which operating points were actually controlled?
Which values were merely observed?
What pressure conditions make the target slow down?
Which software patterns are allowed by evidence?
Which patterns are burst-only?
Which patterns require degraded mode?
Which claims are still blocked?
```

Example output is not just:

```text
Pi4 did 1.19B iterations/sec.
```

A useful `adc-lab` output is closer to:

```text
On a Raspberry Pi 4 target, a 4-worker synthetic CPU load completed for 300s.
Maximum observed temperature was 72.549C under a 75C abort threshold.
Thermal margin was thin.
Governor control was measured for ondemand/performance/powersave.
Fixed-frequency behavior was not measured.
Memory/cache/storage coupling is still insufficient.
Production readiness is blocked.
```

That is the difference between a benchmark and an operating contract.

See [Target Operating Contract architecture](docs/architecture/target-operating-contract.md) for the detailed evidence model.

---

## Relationship to Agent Debug Compass

`agent-debug-compass-laboratory` is separate from Agent Debug Compass Flight Recorder.

| Project                                 | Purpose                                                                                                                          |
| --------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| **Agent Debug Compass Flight Recorder** | Production-oriented, always-on, lightweight evidence preservation around incidents.                                              |
| **ADC Laboratory (`adc-lab`)**          | Explicit experiments, target familiarization, pressure probes, operating-point control, and target operating contract discovery. |

Flight Recorder is the memory of the field system.

Laboratory is the lab bench where an Agent learns what the target can and cannot safely do.

---

## North Star

`adc-lab` is built around these non-negotiable rules:

* **No Agent root shell.**
* **No uncontrolled experiment.**
* **No unapproved irreversible or hard-to-restore operation.**
* **No unqualified tool becomes evidence.**
* **No operating-point claim without controlled or explicitly bounded evidence.**
* **No claim without audit.**
* **No benchmark score without claim boundaries.**

The goal is not to prevent aggressive experiments.

The goal is to make aggressive experiments typed, bounded, approved, audited, restorable, and honest.

---

## Core concepts

| Concept | Meaning |
| ------- | ------- |
| **Controller** | The machine where the Agent or operator runs `adc-lab`, such as a Raspberry Pi 5 controller. |
| **Target** | The machine being measured, such as a Raspberry Pi 4 target. |
| **Target runner** | A non-root `adc-lab-target` helper copied to the target. It exposes fixed subcommands; it is not an arbitrary remote shell. |
| **Privileged helper** | A root-owned fixed-path helper at `/usr/local/libexec/adc-lab-priv-helper` used only for typed privileged operations. It is not a root shell. |
| **Run artifact** | Structured evidence under `lab/runs/<RUN-ID>/`, including manifests, audit logs, inventory, observations, loads, pressure results, and reports. |
| **Target Operating Contract** | A machine-readable contract describing measured mechanisms, boundaries, evidence gaps, and design rules. |
| **Workload demand profile** | Process-scoped demand from a bounded local workload, separated from target-conditioned response and system context. |
| **Suitability decision** | A policy-bound meet / marginal / fail / unknown decision using target run evidence, target contract rules, and workload demand. Unknown never becomes meet. |

---

## Install from GitHub Releases

Use release binaries for Pi4 / Pi5 measurement work.
Do not build from source on the target unless you are explicitly testing the build process.

```sh
VERSION=<latest-release-tag>
# Example:
# VERSION=v0.1.13
ASSET=adc-lab-${VERSION}-linux-aarch64.tar.gz

curl -LO https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${VERSION}/${ASSET}
curl -LO https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${VERSION}/SHA256SUMS

sha256sum -c SHA256SUMS --ignore-missing
tar -xzf "${ASSET}"

bin/adc-lab --version
bin/adc-lab-target --version
bin/adc-lab-priv-helper --version
cat release-manifest.json
```

Release artifacts prove build/package identity.

They do **not** prove resource, NFR, Pi4/Pi5 comparison, target suitability, or production-readiness claims.

More details: [Install release binaries](docs/getting-started/install-release-binaries.md).

### Optional target-local tool and privileged helper install

Most commands do not need the privileged helper. Install it only on a lab
target when governor / operating-point control is explicitly required.

For target-local setup, run the release installer **on the target as the
operator user**, not through `sudo sh`. The installer updates `adc-lab` and
`adc-lab-target` in `~/.local/bin` by default, then installs the privileged
helper at `/usr/local/libexec/adc-lab-priv-helper`.

```sh
ssh <target-host>
curl -fsSLO https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/latest/download/install-adc-lab-helper.sh
bash install-adc-lab-helper.sh --latest --install-sudoers --user "$(id -un)"
```

For reproducible setup, pin a release tag:

```sh
VERSION=vX.Y.Z
curl -fsSLO "https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${VERSION}/install-adc-lab-helper.sh"
bash install-adc-lab-helper.sh --version "${VERSION}" --install-sudoers --user "$(id -un)"
```

If you have the installer checksum from a trusted channel, pin the installer
itself before executing it:

```sh
VERSION=vX.Y.Z
INSTALLER_SHA256=<expected-sha256>
curl -fsSLo install-adc-lab-helper.sh "https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${VERSION}/install-adc-lab-helper.sh"
echo "${INSTALLER_SHA256}  install-adc-lab-helper.sh" | sha256sum -c -
bash install-adc-lab-helper.sh --version "${VERSION}" --install-sudoers --user "$(id -un)"
```

The installer downloads the selected release tarball, verifies it with
`SHA256SUMS`, installs user binaries, installs only
`/usr/local/libexec/adc-lab-priv-helper` for the privileged boundary,
optionally adds a narrow sudoers rule for that exact helper path, and runs
privilege readiness checks.

Do **not** use:

```sh
curl ... | sudo sh
```

Compromised-release protection is still pending. The checksum flow protects
against transfer errors and mismatched assets, but a malicious release could
change both the installer and `SHA256SUMS`. Stronger attestation/signature
verification is a future hardening step.

---

## Quick start: local target

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory
adc-lab load cpu --target local --workers 2 --duration 5s --abort-temp-c 75
adc-lab report pack --run lab/runs/LAB-RUN-...
```

This produces a first evidence pack.
It does not prove target suitability.

After a first run, expect artifacts like:

```text
lab/runs/LAB-RUN-.../
  run_manifest.json
  audit.jsonl
  inventory/target_inventory.json
  toolchain/toolchain_inventory.json
  observations/observe.json
  loads/*.result.json
  reports/claim_evidence_trace.json
```

For the full command reference, see [CLI reference](docs/reference/cli.md).

---

## Quick start: local workload suitability loop

`workload run` v1 is local-target only. It does not run arbitrary workloads over
SSH. For a Pi4 target, run this command on the Pi4 itself, or use SSH only to
invoke the target-local `adc-lab` command and collect artifacts.

```sh
adc-lab workload run \
  --target local \
  --target-id <target-id> \
  --execution-mode target-local \
  --plan examples/workloads/pi4_representative_smoke.yaml \
  --run-dir lab/runs/LAB-RUN-workload-...
```

The representative workload is a safe bounded CPU + RSS + tempfile I/O smoke.
It is exploratory only. It is not real application performance, production
readiness, Pi4/Pi5 selection evidence, sustained thermal safety, or flash-wear
evidence.

SSH workload transport is deliberately deferred:

```sh
adc-lab workload run --target ssh://<target-host> --plan examples/workloads/pi4_representative_smoke.yaml
```

returns a structured refusal with
`reason=remote_workload_execution_not_supported_in_v1`.

After a target operating contract run and a workload run exist:

```sh
adc-lab decide suitability \
  --target-run lab/runs/LAB-RUN-target-contract-... \
  --target-contract lab/runs/LAB-RUN-target-contract-.../reports/target_operating_contract.v2.json \
  --workload-demand lab/runs/LAB-RUN-workload-.../reports/workload_demand_profile.json \
  --policy examples/suitability/pi4-default-policy.yaml \
  --out lab/runs/LAB-RUN-workload-.../reports/suitability_decision.json

adc-lab constraints generate \
  --decision lab/runs/LAB-RUN-workload-.../reports/suitability_decision.json \
  --out lab/runs/LAB-RUN-workload-.../reports/design_constraint_pack.json \
  --agent-instructions-out lab/runs/LAB-RUN-workload-.../reports/agent_constraints.md
```

`agent_constraints.md` is intended to be pasted into an AGENTS.md, CLAUDE.md,
or implementation-agent prompt. It is an instruction artifact, not a benchmark
score.

---

## Quick start: Raspberry Pi 5 controller → Raspberry Pi 4 target

Copy the target runner to the Pi4:

```sh
TARGET_HOST=<pi4-ssh-host>
scp bin/adc-lab-target "${TARGET_HOST}:/home/<target-user>/.local/bin/adc-lab-target"
ssh "${TARGET_HOST}" 'chmod +x /home/<target-user>/.local/bin/adc-lab-target'
ssh "${TARGET_HOST}" '/home/<target-user>/.local/bin/adc-lab-target --version'
```

Then run inventory, toolchain discovery, passive observation, a short bounded load, and report packing from the Pi5 controller.

See [First Pi4 run](docs/getting-started/first-pi4-run.md) for the full step-by-step command sequence.

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH.
It does not expose arbitrary remote shell.

`ADC_LAB_TARGET_RUNNER` is a development override only and must point to an `adc-lab-target` binary from an allowlisted safe path such as:

```text
/usr/local/bin/adc-lab-target
/home/<user>/.local/bin/adc-lab-target
/home/<user>/.local/share/adc-lab/runners/<version>/adc-lab-target
```

---

## Pressure probes

`adc-lab pressure run` and `adc-lab pressure composite` create bounded probe
evidence and v2 `lab.artifact.v2` sidecars for rule evaluation.

Supported pressure kinds:

```text
cpu_pressure
thermal_pressure
memory_pressure
storage_io
network_io
latency_jitter
observer_pressure
```

Pressure probes are command-triggered, bounded, cleanup-aware, artifact-producing, and claim-bounded.

A pressure probe existing does not automatically mean a platform mechanism or coupling effect was fully measured.

For example:

```text
memory allocation smoke != memory pressure boundary
network counter visibility != bounded network transfer boundary
separate memory/storage/jitter probes != composite coupling evidence
storage tempfile smoke != flash safety
idle jitter loop != real-time behavior under pressure
```

See [Pressure probes reference](docs/reference/pressure-probes.md) for command examples and status semantics.

---

## Operating contract reports

Generate a target operating contract from a run:

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id <target-id> \
  --target-class raspberry_pi_4
```

This writes:

```text
lab.artifact.v2 kind=report.operating_contract
```

With `--include-run`, the v2 evidence store opens all provided run
directories and evaluates one combined rule body. It does not emit v1 run-set
or multi-run compatibility artifacts.

The contract describes measured mechanisms, boundary evidence,
resource-coupling evidence class, allowed patterns, burst-only patterns,
degraded-mode triggers, forbidden patterns, blocked claims, and next evidence
needed.

A target operating contract is **not** a benchmark score.

---

## Privileged operating-point control

Privileged control is optional.
Most familiarization and pressure probes can run without installing the privileged helper.

When a test needs to change target state, such as CPU governor control, `adc-lab` does not give the Agent a root shell.

Instead, privileged operations go through:

```text
fixed-path root-owned adc-lab-priv-helper
typed operation plan
approval record
apply result
restore lease
restore verification
audit event
health check
```

For repeated privileged experiments, the recommended workflow is:

1. Operator installs the helper with the release installer or reviewed local
   build.
2. Operator configures the minimal helper-only sudo rule if appropriate.
3. Agent runs `adc-lab` experiments non-interactively.
4. Agent verifies restore and health.
5. Operator removes helper/sudoers when the lab session is done.

See [CLI reference](docs/reference/cli.md#privileged-operating-point-workflow) and [Privilege Model Option A](docs/architecture/privilege-model-option-a.md) for details.

---

## Example: what Pi4 evidence currently looks like

A deep Pi4 run may reveal facts like:

```text
1 worker 60s synthetic CPU load:
  ~299M iter/s

2 worker 60s synthetic CPU load:
  ~599M iter/s

4 worker 60s synthetic CPU load:
  ~1.19B iter/s

4 worker 300s synthetic CPU load:
  completed
  max temp near 72.5C under 75C abort threshold
  thermal margin thin

governor control:
  ondemand and performance are close for synthetic all-core CPU load
  powersave is about one third throughput but much cooler
```

Those are useful facts.

But `adc-lab` will still block claims such as:

```text
Pi4 is production-ready.
Pi4 is sufficient for workload X.
Pi5 is required for workload Y.
Pi4 is safe for 24h sustained thermal load.
All CPU frequencies were measured.
Memory/cache/storage coupling is fully understood.
Network behavior is characterized.
Real-time latency is guaranteed.
```

The point is not to be pessimistic.

The point is to keep the Agent honest.

---

## Common workflows

| Workflow | Start here |
| -------- | ---------- |
| Read-only familiarization | `adc-lab familiarize read-only ...` to learn what the target is and which signals are visible. |
| Bounded CPU load | `adc-lab load cpu ...` to map short CPU / thermal response. |
| Pressure probe | `adc-lab pressure run ...` to create bounded pressure evidence. |
| Operating point experiment | `adc-lab control plan`, approve, apply, load, restore, and health-check. |
| Operating contract generation | `adc-lab report operating-contract ...` after one or more pressure or control runs. |
| Local workload suitability | `adc-lab workload run`, `adc-lab decide suitability`, and `adc-lab constraints generate`. |
| Constraint lint | `adc-lab constraints check ...` to fail on blocked claim text in candidate agent-facing content. |

Local workload suitability decisions can produce meet / marginal / fail /
unknown for one target/workload/policy evidence body, but they still cannot say:

```text
Pi5 is required.
This target is production-ready.
```

See [CLI reference](docs/reference/cli.md) for complete commands.

---

## Evidence model

`adc-lab` separates:

```text
observed covariates
controlled factors
uncontrolled confounders
pressure ingredients
composite coupling evidence
generic lab rules
measured target rules
evidence-needed rules
```

This prevents common mistakes:

```text
Observed CPU frequency range is not a fixed-frequency sweep.

A pressure probe existing is not the same as a measured platform boundary.

Separate memory, storage, and jitter artifacts are ingredients, not proof that memory pressure causes storage latency.

A generic rule is not the same as a measured target-specific rule.
```

---

## What `adc-lab` is not

`adc-lab` is not:

* a generic benchmark suite,
* a root shell wrapper,
* a stress-ng wrapper,
* a production daemon,
* a Flight Recorder replacement,
* a target-selection oracle,
* a root-cause engine,
* a claim generator.

It is a lab system for producing evidence-bound operating contracts.

---

## Current maturity

`adc-lab` is under active development.

Current strengths:

```text
Release binary identity
target inventory
toolchain inventory
passive observation
bounded CPU load
pressure probe artifacts
governor control through typed helper
multi-run operating-contract aggregation
endpoint-backed bounded network transfer
phase-based memory/storage/jitter composite probe
run manifests
audit logs
claim boundaries
operating-contract report skeleton
```

Still being strengthened:

```text
memory pressure boundary discovery
larger memory pressure ladders
concurrent storage I/O under memory pressure
latency/jitter under pressure
fixed-frequency sweep
Pi4/Pi5 comparison
suitability decisions
```

If a field says `insufficient`, that is not a failure.

It means the tool refused to pretend.

---

## Repository layout

```text
crates/
  adc-lab/                 # Controller CLI
  adc-lab-core/            # Contracts, reports, pressure probes, policy logic
  adc-lab-target/          # Non-root target-side runner
  adc-lab-priv-helper/     # Fixed-path privileged helper

schemas/                  # Agent-facing and lab-facing JSON contracts
examples/
  workloads/              # Workload profiles
  experiments/            # Experiment matrix examples
  demos/                  # Target-specific live-run examples

docs/
  architecture/           # Privilege, safety, release, and contract docs
  getting-started/        # First-run guides
  reference/              # CLI reference and command examples

lab/runs/                 # Local run artifacts, ignored by git
```

---

## Verification

Use the repository command wrapper:

```sh
make verify
```

This runs build, format, lint, unit tests, integration tests, contract validation, docs smoke, and command wiring smoke.

The smoke command verifies command wiring only.
It does not by itself support resource, NFR, Pi4/Pi5 comparison, suitability, or production-readiness claims.

---

## Roadmap

Near term:

```text
Improve README and onboarding.
Fix evidence-pack consistency.
Add privilege doctor / install-plan / uninstall-plan.
Add multi-run operating-contract aggregation.
Make pressure run summaries easier to inspect.
```

Pi4 reference contract:

```text
Merge CPU / thermal / governor evidence.
Add memory pressure ladder.
Add storage I/O under memory pressure.
Add latency/jitter under pressure.
Add bounded network transfer.
Add composite coupling probes.
Generate Pi4 Platform Operating Contract v1.
```

Pi4 vs Pi5:

```text
Run the same workload and pressure profiles on Pi4 and Pi5.
Generate target capability profiles.
Generate target comparison reports.
Generate suitability decisions only when evidence supports them.
```

Future targets:

```text
Jetson
Snapdragon / Android
Mac mini
generic embedded Linux
ROS 2 robots
```

Platform-specific mechanisms are adapter-specific.

The core model stays generic:

```text
measure raw capability
discover platform mechanisms
find boundary conditions
derive target operating contracts
constrain software design
```

---

## Design philosophy

`adc-lab` is built around one idea:

> A target is not characterized until its operating contract is known.

That means:

```text
not just what the hardware can do,
but what software must respect to keep it fast, stable, cool, responsive, and recoverable.
```

This is the layer an AI agent needs before it can responsibly design software for real embedded and edge devices.

---

## Status

This project is pre-1.0.

APIs, schemas, and report formats may change.

The guiding rule will not:

```text
No evidence, no claim.
```
