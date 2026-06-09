# Getting Started: Pi5 Controller To Pi4 Target

Build on the controller:

```sh
make build-release
```

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

Run the read-only familiarization pack smoke. This performs no privileged
control, no cpufreq writes, no load generation, and only qualifies discovered
tools through inventory policy:

```sh
TARGET=ssh://pi4 scripts/targets/pi5-to-pi4-readonly-familiarization.sh
```

The smoke writes one run directory containing `run_manifest.json`,
`reports/familiarization_pack.json`, `reports/claim_evidence_trace.json`,
`inventory/target_inventory.json`, `toolchain/toolchain_inventory.json`,
`tools/tool_qualification_summary.json`, `observations/observe.json`, and
`audit.jsonl`.

Privileged control requires an approval artifact and uses the helper. In this
MVP, privileged apply/restore is local-target only; remote privileged apply is
deferred until a target-local helper transport is implemented. Do not grant an
agent a root shell.

Local-only dry-run workflow:

```sh
adc-lab control plan --target local cpu.governor --set performance
adc-lab control approve --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approved-by operator
adc-lab control apply --plan lab/runs/LAB-RUN-.../plans/PLAN-....json --approval lab/runs/LAB-RUN-.../approvals/APPROVAL-....json --dry-run
```

Remove `--dry-run` only after the fixed helper is installed on the local lab
target and the operator has reviewed the approval artifact.
