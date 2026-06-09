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

Privileged control requires an approval artifact and uses the helper. In this MVP, privileged apply/restore is local-target only; remote privileged apply is deferred until a target-local helper transport is implemented. Do not grant an agent a root shell.
