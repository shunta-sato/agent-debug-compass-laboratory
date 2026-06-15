# Getting Started: Pi5 Controller To Pi4 Target

For Pi4/Pi5 measurements that need the same binary identity, install from a
GitHub Release and verify `SHA256SUMS` first. See
`docs/getting-started/install-release-binaries.md`.

Release-binary preflight:

```sh
sha256sum -c SHA256SUMS
tar -xzf adc-lab-v0.1.0-linux-aarch64.tar.gz
bin/adc-lab --version
bin/adc-lab-target --version
cat release-manifest.json
```

Build on the controller:

```sh
make build-release
```

The source-build path above is for local development. Same-suite Pi4/Pi5
measurement prompts should prefer release binaries and record `--version` plus
`release-manifest.json`.

Install the helper on a lab target after reviewing the script:

```sh
scripts/install-helper.sh target/release/adc-lab-priv-helper
```

Run a local smoke on the controller:

```sh
adc-lab inventory --target local
adc-lab toolchain discover --target local
adc-lab observe --target local --duration 5s --signals cpu,freq,thermal,memory
```

Run against a Pi4 target with `adc-lab-target` available on PATH:

```sh
adc-lab inventory --target ssh://pi4
adc-lab toolchain discover --target ssh://pi4
adc-lab observe --target ssh://pi4 --duration 30s --signals cpu,freq,thermal,memory
```

If `adc-lab-target` was installed by the release installer under
`~/.local/bin`, non-interactive SSH may not include that directory in PATH. Use
the fixed runner path instead of relying on remote PATH lookup:

```sh
ADC_LAB_TARGET_RUNNER=/home/<target-user>/.local/bin/adc-lab-target \
  adc-lab inventory --target ssh://pi4
```

Run the read-only familiarization pack smoke. This performs no privileged
control, no cpufreq writes, no load generation, and only qualifies discovered
tools through inventory policy:

```sh
TARGET=ssh://pi4 scripts/targets/pi5-to-pi4-readonly-familiarization.sh
```

The smoke writes one run directory containing `run_manifest.json`,
`reports/run_report.v2.json`, `inventory/target_inventory.json`,
`toolchain/toolchain_inventory.json`, `tools/tool_qualification_summary.json`,
`observations/observe.json`, and `audit.jsonl`.

Optional Tier 1 bounded CPU load is separate from read-only familiarization.
Keep it short, set a thermal abort when the target has a thermal surface, and
use a target-local operator abort marker:

```sh
adc-lab load cpu --target ssh://pi4 --workers 2 --duration 30s --abort-temp-c 75 --operator-abort-file /tmp/adc-lab-abort
```

Creating `/tmp/adc-lab-abort` on the target stops the load and records
`status=aborted` with `abort_reason=operator_abort`. The abort marker path is
runtime input only and is not serialized into load artifacts.

Privileged control requires an approval artifact and uses the helper. In this
MVP, privileged apply/restore is local-target only; remote privileged apply is
deferred until a target-local helper transport is implemented. Do not grant an
agent a root shell.

For full-set governor evidence, prefer the high-level sweep workflow in
`docs/reference/cli.md#governor-sweep-workflow`: prepare a sweep policy,
approve that policy out of band, run the sweep, then validate the run with
`adc-lab report validate-run`. The single-plan flow below is for one reviewed
control operation, not for selecting full-set plan or approval artifacts.

Local-only dry-run workflow:

```sh
adc-lab control plan --target local cpu.governor --set performance
adc-lab control approve --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approved-by operator
adc-lab control apply --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approval lab/runs/LAB-RUN-.../approvals/APPROVAL-....json --dry-run
```

Remove `--dry-run` only after the fixed helper is installed on the local lab
target and the operator has reviewed the approval artifact.
