# Install Release Binaries

Use GitHub Release binaries when Pi4/Pi5 measurements must use the same
`adc-lab` commit, release, and checksum. Source builds are useful for
development, but they are not the default measurement handoff path.

## Download And Verify

Download the release tarball for the controller or target architecture plus
`SHA256SUMS` from the GitHub Release page.

For Raspberry Pi 4 and Raspberry Pi 5, use the aarch64 asset:

```sh
sha256sum -c SHA256SUMS
tar -xzf adc-lab-v0.1.0-linux-aarch64.tar.gz
```

For x86_64 developer hosts:

```sh
sha256sum -c SHA256SUMS
tar -xzf adc-lab-v0.1.0-linux-x86_64.tar.gz
```

Inspect the release manifest:

```sh
cat release-manifest.json
bin/adc-lab --version
bin/adc-lab-target --version
bin/adc-lab-priv-helper --version
```

If GitHub artifact attestations are enabled for the release, verify provenance
with GitHub CLI:

```sh
gh attestation verify adc-lab-v0.1.0-linux-aarch64.tar.gz -R <owner>/<repo>
```

The release manifest and `--version` output are build/package integrity
evidence. They are not resource, NFR, Pi4/Pi5 comparison, or production
readiness evidence.

## Controller Install

On the Pi5 controller:

```sh
mkdir -p ~/.local/bin
cp bin/adc-lab ~/.local/bin/
chmod +x ~/.local/bin/adc-lab
adc-lab --version
```

## Target Runner Install

On the controller, copy the non-root target runner to the Pi4 target:

```sh
ssh pi4 'mkdir -p ~/.local/bin'
scp bin/adc-lab-target pi4:~/.local/bin/
ssh pi4 'chmod +x ~/.local/bin/adc-lab-target && ~/.local/bin/adc-lab-target --version'
```

Then run read-only commands with the fixed runner path:

```sh
ADC_LAB_TARGET_RUNNER=/home/$USER/.local/bin/adc-lab-target \
  adc-lab inventory --target ssh://pi4
```

Replace `/home/$USER` with the target user's home directory when it differs
from the controller user.

## Optional Privileged Helper

Most measurements are non-privileged by default. Install the helper only when a
privileged-control workflow explicitly requires it and the operator has reviewed
the release version and checksum boundary.

Preferred target-local release installer:

```sh
ADC_LAB_VERSION=v0.1.13 bash -c 'set -euo pipefail; tmp="$(mktemp -d)"; cd "$tmp"; curl -fsSLO "https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${ADC_LAB_VERSION}/install-adc-lab-helper.sh"; bash install-adc-lab-helper.sh --version "${ADC_LAB_VERSION}" --install-sudoers --user "$(id -un)"'
```

Checksum-pinned variant, when the installer hash is available from a trusted
channel:

```sh
ADC_LAB_VERSION=v0.1.13 INSTALLER_SHA256=<expected-sha256> bash -c 'set -euo pipefail; tmp="$(mktemp -d)"; cd "$tmp"; curl -fsSLo install-adc-lab-helper.sh "https://github.com/shunta-sato/agent-debug-compass-laboratory/releases/download/${ADC_LAB_VERSION}/install-adc-lab-helper.sh"; echo "${INSTALLER_SHA256}  install-adc-lab-helper.sh" | sha256sum -c -; bash install-adc-lab-helper.sh --version "${ADC_LAB_VERSION}" --install-sudoers --user "$(id -un)"'
```

The installer runs as the operator user, not as root. It uses `sudo` only for
the fixed helper install path and optional fixed sudoers file. It downloads the
pinned release tarball, verifies it with `SHA256SUMS`, installs
`/usr/local/libexec/adc-lab-priv-helper`, and runs readiness checks.

Manual install from an already verified/extracted tarball remains:

```sh
sudo install -o root -g root -m 0755 bin/adc-lab-priv-helper /usr/local/libexec/adc-lab-priv-helper
/usr/local/libexec/adc-lab-priv-helper --version
```

Do not grant an agent a root shell. The helper remains an allowlisted typed
operation boundary.

Do not use `curl | sudo sh`. Compromised-release protection is still pending;
checksums protect against mismatched assets, not a malicious release that
changes both the installer and `SHA256SUMS`.
