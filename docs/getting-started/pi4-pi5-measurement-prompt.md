# Pi4/Pi5 Measurement Prompt

Before collecting Pi4/Pi5 capability evidence, install `adc-lab` from a GitHub
Release and verify checksums. Do not use a source build for same-binary
comparison unless the run is explicitly marked as development-only.

## Required Preflight

```sh
sha256sum -c SHA256SUMS
tar -xzf adc-lab-v0.1.0-linux-aarch64.tar.gz
bin/adc-lab --version
bin/adc-lab-target --version
cat release-manifest.json
```

Record the `--version` JSON and `release-manifest.json` in the later run
manifest and v2 evidence artifacts. Do not reuse v1 demo packs for v2
comparison evidence.

## Pi5 Controller To Pi4 Target

```sh
mkdir -p ~/.local/bin
cp bin/adc-lab ~/.local/bin/

ssh pi4 'mkdir -p ~/.local/bin'
scp bin/adc-lab-target pi4:~/.local/bin/
ssh pi4 'chmod +x ~/.local/bin/adc-lab-target && ~/.local/bin/adc-lab-target --version'

ADC_LAB_TARGET_RUNNER=/home/$USER/.local/bin/adc-lab-target \
  TARGET=ssh://pi4 \
  scripts/targets/pi5-to-pi4-readonly-familiarization.sh
```

This prompt establishes binary identity and read-only familiarization only. It
does not support claims that Pi4 is sufficient, Pi5 is required, battery usage
is safe, sustained thermal behavior is safe, or all operating points were
measured.
