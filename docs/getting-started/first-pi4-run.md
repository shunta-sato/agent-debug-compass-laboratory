# First Pi4 Run From A Pi5 Controller

This guide is the longer first-run path behind the README quick start. It assumes a Raspberry Pi 5 controller and a Raspberry Pi 4 target reachable over SSH.

The goal is to produce a first evidence pack. The goal is not to prove production readiness, target suitability, or Pi4/Pi5 superiority.

## 1. Install release binaries on the controller

Use release binaries for Pi4 / Pi5 measurement work. Do not build from source on the target unless you are explicitly testing the build process.

```sh
VERSION=vX.Y.Z[.N]
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

Release artifacts prove build/package identity only. They do not prove resource, NFR, Pi4/Pi5 comparison, target suitability, or production-readiness claims.

## 2. Copy the non-root target runner to the Pi4

```sh
TARGET_HOST=<pi4-ssh-host>
TARGET_USER=<target-user>

ssh "${TARGET_HOST}" "mkdir -p /home/${TARGET_USER}/.local/bin"
scp bin/adc-lab-target "${TARGET_HOST}:/home/${TARGET_USER}/.local/bin/adc-lab-target"
ssh "${TARGET_HOST}" "chmod +x /home/${TARGET_USER}/.local/bin/adc-lab-target"
ssh "${TARGET_HOST}" "/home/${TARGET_USER}/.local/bin/adc-lab-target --version"
```

The target runner exposes fixed subcommands for inventory, observation, health checks, and bounded non-root experiments. It is not an arbitrary remote shell.

## 3. Point the controller at the target runner

```sh
export ADC_LAB_TARGET_RUNNER="/home/${TARGET_USER}/.local/bin/adc-lab-target"
export ADC_LAB_TARGET="ssh://${TARGET_HOST}"
export ADC_LAB_TARGET_ID=<target-id>
```

`ADC_LAB_TARGET_RUNNER` is a development override only and must point to an `adc-lab-target` binary from an allowlisted safe path such as:

```text
/usr/local/bin/adc-lab-target
/home/<user>/.local/bin/adc-lab-target
/home/<user>/.local/share/adc-lab/runners/<version>/adc-lab-target
```

## 4. Collect inventory and toolchain evidence

```sh
adc-lab inventory \
  --target "${ADC_LAB_TARGET}" \
  --run-dir lab/runs/pi4-smoke \
  --json

adc-lab toolchain discover \
  --target "${ADC_LAB_TARGET}" \
  --run-dir lab/runs/pi4-smoke \
  --json
```

These commands establish what the target is and which toolchain facts were discovered. Tool discovery is not the same as qualifying every tool for every claim.

## 5. Observe passive target signals

```sh
adc-lab observe \
  --target "${ADC_LAB_TARGET}" \
  --duration 60s \
  --signals cpu,freq,thermal,memory \
  --run-dir lab/runs/pi4-smoke \
  --json
```

Passive observation records visible covariates. Observed CPU frequency variation is not a fixed-frequency sweep.

## 6. Run a short bounded CPU load

```sh
adc-lab load cpu \
  --target "${ADC_LAB_TARGET}" \
  --workers 2 \
  --duration 60s \
  --abort-temp-c 75 \
  --run-dir lab/runs/pi4-smoke \
  --json
```

Keep first loads short. Use a thermal abort when the target has a thermal surface. A short CPU load is useful evidence, but it does not prove 24-hour thermal safety or production readiness.

## 7. Pack the run

```sh
adc-lab report pack \
  --run lab/runs/pi4-smoke \
  --target-id "${ADC_LAB_TARGET_ID}" \
  --target "${ADC_LAB_TARGET}" \
  --json
```

Typical artifacts include:

```text
run_manifest.json
audit.jsonl
inventory/target_inventory.json
toolchain/toolchain_inventory.json
observations/observe.json
load/*.v2.json
reports/run_report.v2.json
```

## 8. Optional: generate an operating contract

After one or more pressure or control runs, generate a target operating contract:

```sh
adc-lab report operating-contract \
  --run lab/runs/pi4-smoke \
  --target-id "${ADC_LAB_TARGET_ID}" \
  --target-class raspberry_pi_4
```

The contract can describe measured mechanisms, boundary evidence, resource-coupling evidence class, allowed patterns, burst-only patterns, degraded-mode triggers, forbidden patterns, blocked claims, and next evidence needed.

A target operating contract is not a benchmark score.

## What this first run can and cannot prove

A first Pi4 smoke run can support claims such as:

```text
The target runner executed fixed adc-lab-target subcommands over SSH.
Inventory and toolchain evidence were captured.
Passive CPU/frequency/thermal/memory observations were captured.
A bounded CPU load completed or aborted with recorded safety evidence.
A run pack was generated with audit evidence.
```

It cannot support claims such as:

```text
Pi4 is production-ready.
Pi4 is sufficient for workload X.
Pi5 is required for workload Y.
Pi4 is safe for 24h sustained thermal load.
Memory/cache/storage coupling is fully understood.
Network behavior is characterized.
Real-time latency is guaranteed.
```

No evidence, no claim.
