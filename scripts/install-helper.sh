#!/usr/bin/env bash
set -euo pipefail

helper="${1:-target/release/adc-lab-priv-helper}"
dest="/usr/local/libexec/adc-lab-priv-helper"

if [[ ! -x "$helper" ]]; then
  echo "helper binary is missing or not executable: $helper" >&2
  echo "build first with: make build-release" >&2
  exit 2
fi

echo "Installing adc-lab privileged helper to ${dest}"
sudo install -o root -g root -m 0755 "$helper" "$dest"

cat <<'MSG'
Installed helper with normal sudo prompting.

adc-lab does not require NOPASSWD sudoers for MVP.
If a lab machine later needs restricted sudoers, review and install a narrow rule out of band:

  %adc-lab ALL=(root) /usr/local/libexec/adc-lab-priv-helper

Do not grant an agent a root shell.
MSG
