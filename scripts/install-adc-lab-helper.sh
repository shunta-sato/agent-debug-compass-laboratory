#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY="shunta-sato/agent-debug-compass-laboratory"
readonly HELPER_DEST="/usr/local/libexec/adc-lab-priv-helper"
readonly SUDOERS_DEST="/etc/sudoers.d/adc-lab"

version=""
asset_triple="auto"
install_sudoers="false"
sudo_user=""
keep_temp="false"

usage() {
  cat <<'USAGE'
usage: install-adc-lab-helper.sh --version vX.Y.Z [options]

Install the adc-lab privileged helper from a pinned GitHub Release tarball.

Options:
  --version vX.Y.Z        Required release tag to install.
  --asset-triple VALUE    Optional release asset triple. Defaults to auto.
                          Supported: linux-aarch64, linux-x86_64.
  --install-sudoers      Also install a narrow NOPASSWD sudoers rule.
  --user USER             Required with --install-sudoers; must be current user.
  --keep-temp             Keep the temporary download directory for inspection.
  -h, --help              Show this help.

This script must run as a normal user. It never starts an arbitrary root shell.
It uses sudo only for the fixed helper install path and optional fixed sudoers
file.
USAGE
}

die() {
  echo "error: $*" >&2
  exit 2
}

require_command() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

validate_version() {
  local value="$1"
  if [[ ! "$value" =~ ^v[0-9]+[.][0-9]+[.][0-9]+([._+-][A-Za-z0-9._+-]+)?$ ]]; then
    die "invalid --version: $value"
  fi
}

validate_sudo_user() {
  local value="$1"
  local current_user="$2"
  if [[ ! "$value" =~ ^[A-Za-z_][A-Za-z0-9_-]*[$]?$ ]]; then
    die "invalid --user: $value"
  fi
  if [[ "$value" != "$current_user" ]]; then
    die "--user must match current user for non-interactive verification: $current_user"
  fi
}

detect_asset_triple() {
  local kernel
  local machine
  kernel="$(uname -s)"
  machine="$(uname -m)"
  if [[ "$kernel" != "Linux" ]]; then
    die "unsupported OS for helper install: $kernel"
  fi
  case "$machine" in
    aarch64|arm64)
      echo "linux-aarch64"
      ;;
    x86_64|amd64)
      echo "linux-x86_64"
      ;;
    *)
      die "unsupported CPU architecture for helper install: $machine"
      ;;
  esac
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="${2:?missing --version value}"
      shift 2
      ;;
    --asset-triple)
      asset_triple="${2:?missing --asset-triple value}"
      shift 2
      ;;
    --install-sudoers)
      install_sudoers="true"
      shift
      ;;
    --user)
      sudo_user="${2:?missing --user value}"
      shift 2
      ;;
    --keep-temp)
      keep_temp="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[[ -n "$version" ]] || die "missing required --version"
validate_version "$version"

if [[ "$(id -u)" -eq 0 ]]; then
  die "do not run this installer as root; run as the target operator user"
fi

if [[ "$asset_triple" == "auto" ]]; then
  asset_triple="$(detect_asset_triple)"
fi
case "$asset_triple" in
  linux-aarch64|linux-x86_64)
    ;;
  *)
    die "unsupported --asset-triple: $asset_triple"
    ;;
esac

current_user="$(id -un)"
if [[ "$install_sudoers" == "true" ]]; then
  [[ -n "$sudo_user" ]] || die "--user is required with --install-sudoers"
  validate_sudo_user "$sudo_user" "$current_user"
fi

require_command curl
require_command grep
require_command id
require_command install
require_command mktemp
require_command sha256sum
require_command sudo
require_command tar
require_command uname
if [[ "$install_sudoers" == "true" ]]; then
  require_command visudo
fi

tmp_dir="$(mktemp -d)"
cleanup() {
  if [[ "$keep_temp" == "true" ]]; then
    echo "kept temporary directory: $tmp_dir" >&2
  else
    rm -rf "$tmp_dir"
  fi
}
trap cleanup EXIT

asset="adc-lab-${version}-${asset_triple}.tar.gz"
base_url="https://github.com/${REPOSITORY}/releases/download/${version}"
extract_dir="${tmp_dir}/extract"

echo "Downloading ${asset} from ${base_url}" >&2
(
  cd "$tmp_dir"
  curl -fsSLO "${base_url}/${asset}"
  curl -fsSLO "${base_url}/SHA256SUMS"
  grep -F " ${asset}" SHA256SUMS >/dev/null || die "SHA256SUMS does not list ${asset}"
  sha256sum -c SHA256SUMS --ignore-missing
)

mkdir -p "$extract_dir"
tar -xzf "${tmp_dir}/${asset}" -C "$extract_dir"

helper="${extract_dir}/bin/adc-lab-priv-helper"
adc_lab="${extract_dir}/bin/adc-lab"
manifest="${extract_dir}/release-manifest.json"
version_no_v="${version#v}"

[[ -x "$helper" ]] || die "release tarball does not contain executable bin/adc-lab-priv-helper"
[[ -f "$manifest" ]] || die "release tarball does not contain release-manifest.json"
grep -F "\"version\": \"${version_no_v}\"" "$manifest" >/dev/null \
  || die "release manifest version does not match ${version}"
grep -F '"name": "adc-lab-priv-helper"' "$manifest" >/dev/null \
  || die "release manifest does not list adc-lab-priv-helper"

"$helper" --version >/dev/null

echo "Installing helper to ${HELPER_DEST}" >&2
sudo install -o root -g root -m 0755 "$helper" "$HELPER_DEST"
"$HELPER_DEST" --version >/dev/null

if [[ "$install_sudoers" == "true" ]]; then
  sudoers_tmp="${tmp_dir}/adc-lab-sudoers"
  printf '%s ALL=(root) NOPASSWD: %s\n' "$sudo_user" "$HELPER_DEST" > "$sudoers_tmp"
  chmod 0440 "$sudoers_tmp"
  sudo visudo -cf "$sudoers_tmp" >/dev/null
  echo "Installing sudoers rule to ${SUDOERS_DEST}" >&2
  sudo install -o root -g root -m 0440 "$sudoers_tmp" "$SUDOERS_DEST"
  sudo -n "$HELPER_DEST" --version >/dev/null
fi

if [[ -x "$adc_lab" ]]; then
  "$adc_lab" privilege doctor \
    --target local \
    --run-dir "${tmp_dir}/privilege-doctor-run" \
    --json
else
  echo "installed ${HELPER_DEST}" >&2
fi
