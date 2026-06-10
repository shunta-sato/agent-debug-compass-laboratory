# agent-debug-compass-laboratory

```text
                 _            _       _
   __ _  __| | ___       | | __ _| |__
  / _` |/ _` |/ __|_____| |/ _` | '_ \
 | (_| | (_| | (_|_____| | (_| | |_) |
  \__,_|\__,_|\___|     |_|\__,_|_.__/

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

## Why this exists

AI agents are getting good at writing software.

They are still bad at understanding the physical reality of the target they are writing for.

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
On target55, Raspberry Pi 4 completed a 4-worker synthetic CPU load for 300s.
Maximum observed temperature was 72.549C under a 75C abort threshold.
Thermal margin was thin.
Governor control was measured for ondemand/performance/powersave.
Fixed-frequency behavior was not measured.
Memory/cache/storage coupling is still insufficient.
Production readiness is blocked.
```

That is the difference between a benchmark and an operating contract.

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

### Controller

The machine where the Agent or operator runs `adc-lab`.

Example:

```text
Raspberry Pi 5 controller
```

### Target

The machine being measured.

Example:

```text
Raspberry Pi 4 target
```

### Target runner

A non-root helper copied to the target:

```text
adc-lab-target
```

It exposes fixed subcommands for inventory, observation, health checks, and bounded non-root experiments.

It is not an arbitrary remote shell.

### Privileged helper

A root-owned, fixed-path helper used only for typed privileged operations:

```text
/usr/local/libexec/adc-lab-priv-helper
```

It is used for operations like approved governor control.

It is not a root shell.

### Run artifact

Each evidence-producing run writes structured artifacts under:

```text
lab/runs/<RUN-ID>/
```

Typical artifacts include:

```text
run_manifest.json
audit.jsonl
inventory/target_inventory.json
toolchain/toolchain_inventory.json
observations/observe.json
loads/*.result.json
pressure/*.result.json
reports/platform_mechanism_inventory.json
reports/resource_coupling_report.json
reports/target_operating_contract.json
```

### Target Operating Contract

A machine-readable contract describing the target’s measured mechanisms, boundaries, evidence gaps, and design rules.

It is generated by:

```sh
adc-lab report operating-contract ...
```

---

## Install from GitHub Releases

Use release binaries for Pi4 / Pi5 measurement work.
Do not build from source on the target unless you are explicitly testing the build process.

```sh
VERSION=v0.1.13
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

For the full command reference, see [CLI reference](docs/reference/cli.md).

---

## Quick start: Raspberry Pi 5 controller → Raspberry Pi 4 target

Copy the target runner to the Pi4:

```sh
scp bin/adc-lab-target target55:/home/<user>/.local/bin/adc-lab-target
ssh target55 'chmod +x /home/<user>/.local/bin/adc-lab-target'
ssh target55 '/home/<user>/.local/bin/adc-lab-target --version'
```

Then run from the Pi5 controller:

```sh
export ADC_LAB_TARGET_RUNNER=/home/<user>/.local/bin/adc-lab-target

adc-lab inventory --target ssh://target55 --run-dir lab/runs/pi4-smoke --json

adc-lab toolchain discover --target ssh://target55 --run-dir lab/runs/pi4-smoke --json

adc-lab observe \
  --target ssh://target55 \
  --duration 60s \
  --signals cpu,freq,thermal,memory \
  --run-dir lab/runs/pi4-smoke \
  --json

adc-lab load cpu \
  --target ssh://target55 \
  --workers 2 \
  --duration 60s \
  --abort-temp-c 75 \
  --run-dir lab/runs/pi4-smoke \
  --json

adc-lab report pack \
  --run lab/runs/pi4-smoke \
  --target-id target55 \
  --target ssh://target55 \
  --json
```

For SSH targets, `adc-lab` uses fixed `adc-lab-target` subcommands over SSH.
It does not expose arbitrary remote shell.

`ADC_LAB_TARGET_RUNNER` is a development override only and must point to an `adc-lab-target` binary from an allowlisted safe path such as:

```text
/usr/local/bin/adc-lab-target
/home/<user>/.local/bin/adc-lab-target
/home/<user>/.local/share/adc-lab/runners/<version>/adc-lab-target
```

More details: [First Pi4 run](docs/getting-started/first-pi4-run.md).

---

## Pressure probes

`adc-lab pressure run` creates bounded `lab.resource_pressure_result.v1` artifacts.

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

Examples:

```sh
adc-lab pressure run --target local --kind latency_jitter --duration 1s

adc-lab pressure run \
  --target local \
  --kind memory_pressure \
  --duration 1s \
  --memory-bytes 8388608

adc-lab pressure run \
  --target local \
  --kind storage_io \
  --duration 1s \
  --storage-bytes 1048576

adc-lab pressure run --target local --kind network_io --duration 1s
adc-lab pressure run --target local --kind observer_pressure --duration 1s
```

Pressure probes are:

```text
command-triggered
bounded
cleanup-aware
artifact-producing
claim-bounded
```

A pressure probe existing does not automatically mean a platform mechanism or coupling effect was fully measured.

For example:

```text
memory allocation smoke != memory pressure boundary
network counter visibility != network operating contract
storage tempfile smoke != flash safety
idle jitter loop != real-time behavior under pressure
```

---

## Operating contract reports

Generate a target operating contract from a run:

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --target-class raspberry_pi_4
```

This writes:

```text
lab.platform_mechanism_inventory.v1
lab.boundary_probe_plan.v1
lab.resource_coupling_report.v1
lab.target_operating_contract.v1
```

The contract describes:

```text
measured mechanisms
boundary evidence
resource-coupling evidence class
allowed patterns
burst-only patterns
degraded-mode triggers
forbidden patterns
blocked claims
next evidence needed
```

A target operating contract is **not** a benchmark score.

---

## Privileged operating-point control

Some experiments require changing target state, such as CPU governor control.

`adc-lab` does not give the Agent a root shell.

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

Typical workflow:

```sh
adc-lab control plan --target local cpu.governor --set performance

adc-lab control approve \
  --plan lab/runs/LAB-RUN-.../plans/PLAN-....json \
  --approved-by operator

adc-lab control apply \
  --plan lab/runs/LAB-RUN-.../plans/PLAN-....json \
  --approval lab/runs/LAB-RUN-.../approvals/APPROVAL-....json

adc-lab restore \
  --lease lab/runs/LAB-RUN-.../leases/LEASE-....json

adc-lab health-check --target local
```

For repeated privileged experiments, the recommended workflow is:

1. Operator installs the helper.
2. Operator configures the minimal helper-only sudo rule if appropriate.
3. Agent runs `adc-lab` experiments non-interactively.
4. Agent verifies restore and health.
5. Operator removes helper/sudoers when the lab session is done.

Future work will make this smoother with a privilege doctor / install-plan / uninstall-plan flow.

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

### 1. Read-only familiarization

Use this when you only want to learn what the target is and what signals are visible.

```sh
adc-lab familiarize read-only \
  --target ssh://target55 \
  --duration 60s \
  --signals cpu,freq,thermal,memory
```

Output:

```text
target inventory
toolchain inventory
passive observations
run manifest
audit log
claim boundary
```

### 2. Bounded CPU load

Use this to map short CPU / thermal response.

```sh
adc-lab load cpu \
  --target ssh://target55 \
  --workers 4 \
  --duration 60s \
  --abort-temp-c 75
```

### 3. Operating point experiment

Use this when you need to know how a platform policy changes behavior.

```sh
adc-lab control plan --target local cpu.governor --set performance
adc-lab control approve --plan ... --approved-by operator
adc-lab control apply --plan ... --approval ...
adc-lab load cpu --target local --workers 4 --duration 120s --abort-temp-c 75
adc-lab restore --lease ...
adc-lab health-check --target local
```

### 4. Operating contract generation

Use this after one or more pressure or control runs.

```sh
adc-lab report operating-contract \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --target-class raspberry_pi_4
```

### 5. Workload capability profile

Use this to bind a workload definition to a run.

```sh
adc-lab report capability-profile \
  --run lab/runs/LAB-RUN-... \
  --target-id target55 \
  --workload examples/workloads/bounded_cpu_load_2_workers_60s.json
```

Capability profiles are exploratory until comparison and suitability contracts are added.

They cannot say:

```text
Pi4 is sufficient.
Pi5 is required.
This target is production-ready.
```

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
run manifests
audit logs
claim boundaries
operating-contract report skeleton
```

Still being strengthened:

```text
multi-run aggregation
composite resource-coupling probes
memory pressure boundary discovery
storage I/O under memory pressure
network bounded transfer
latency/jitter under pressure
fixed-frequency sweep
privileged helper UX
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

This runs:

```text
build
format
lint
unit tests
integration tests
contract validation
docs smoke
command wiring smoke
```

The smoke command verifies command wiring only.

It does not by itself support resource, NFR, Pi4/Pi5 comparison, suitability, or production-readiness claims.

---

## Roadmap

### Near term

```text
Improve README and onboarding.
Fix evidence-pack consistency.
Add privilege doctor / install-plan / uninstall-plan.
Add multi-run operating-contract aggregation.
Make pressure run summaries easier to inspect.
```

### Pi4 reference contract

```text
Merge CPU / thermal / governor evidence.
Add memory pressure ladder.
Add storage I/O under memory pressure.
Add latency/jitter under pressure.
Add bounded network transfer.
Add composite coupling probes.
Generate Pi4 Platform Operating Contract v1.
```

### Pi4 vs Pi5

```text
Run the same workload and pressure profiles on Pi4 and Pi5.
Generate target capability profiles.
Generate target comparison reports.
Generate suitability decisions only when evidence supports them.
```

### Future targets

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
