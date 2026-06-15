#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY="shunta-sato/agent-debug-compass-laboratory"
readonly HELPER_DEST="/usr/local/libexec/adc-lab-priv-helper"
readonly SUDOERS_DEST="/etc/sudoers.d/adc-lab"

version=""
use_latest="false"
asset_triple="auto"
install_sudoers="false"
sudo_user=""
keep_temp="false"
install_user_bins="true"
user_bin_dir="${HOME}/.local/bin"

usage() {
  cat <<'USAGE'
usage: install-adc-lab-helper.sh (--version vX.Y.Z[.N] | --latest) [options]

Install adc-lab user binaries and the privileged helper from a GitHub Release.

Options:
  --version vX.Y.Z[.N]    Pinned release tag to install.
  --latest                Install from the GitHub latest release pointer.
  --asset-triple VALUE    Optional release asset triple. Defaults to auto.
                          Supported: linux-aarch64, linux-x86_64.
  --user-bin-dir DIR      Install adc-lab and adc-lab-target here.
                          Defaults to ~/.local/bin.
  --no-user-bins          Do not install adc-lab or adc-lab-target.
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

validate_user_bin_dir() {
  local value="$1"
  if [[ -z "$value" || "$value" != /* ]]; then
    die "--user-bin-dir must be an absolute path"
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
    --latest)
      use_latest="true"
      shift
      ;;
    --asset-triple)
      asset_triple="${2:?missing --asset-triple value}"
      shift 2
      ;;
    --user-bin-dir)
      user_bin_dir="${2:?missing --user-bin-dir value}"
      shift 2
      ;;
    --no-user-bins)
      install_user_bins="false"
      shift
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

if [[ -n "$version" && "$use_latest" == "true" ]]; then
  die "--version and --latest are mutually exclusive"
fi
if [[ -z "$version" && "$use_latest" != "true" ]]; then
  die "missing required --version or --latest"
fi
if [[ -n "$version" ]]; then
  validate_version "$version"
fi
if [[ "$install_user_bins" == "true" ]]; then
  validate_user_bin_dir "$user_bin_dir"
fi

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
require_command sed
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

if [[ "$use_latest" == "true" ]]; then
  base_url="https://github.com/${REPOSITORY}/releases/latest/download"
else
  base_url="https://github.com/${REPOSITORY}/releases/download/${version}"
fi
extract_dir="${tmp_dir}/extract"

echo "Downloading SHA256SUMS from ${base_url}" >&2
(
  cd "$tmp_dir"
  curl -fsSLO "${base_url}/SHA256SUMS"
)

if [[ "$use_latest" == "true" ]]; then
  asset="$(sed -nE "s/^[[:xdigit:]]{64}[[:space:]]+(adc-lab-v[^[:space:]]+-${asset_triple}[.]tar[.]gz)$/\\1/p" "${tmp_dir}/SHA256SUMS" | head -n 1)"
  [[ -n "$asset" ]] || die "SHA256SUMS does not list a latest ${asset_triple} tarball"
else
  asset="adc-lab-${version}-${asset_triple}.tar.gz"
fi

echo "Downloading ${asset} from ${base_url}" >&2
(
  cd "$tmp_dir"
  curl -fsSLO "${base_url}/${asset}"
  grep -F " ${asset}" SHA256SUMS >/dev/null || die "SHA256SUMS does not list ${asset}"
  sha256sum -c SHA256SUMS --ignore-missing
)

mkdir -p "$extract_dir"
tar -xzf "${tmp_dir}/${asset}" -C "$extract_dir"

helper="${extract_dir}/bin/adc-lab-priv-helper"
adc_lab="${extract_dir}/bin/adc-lab"
adc_lab_target="${extract_dir}/bin/adc-lab-target"
manifest="${extract_dir}/release-manifest.json"

[[ -x "$helper" ]] || die "release tarball does not contain executable bin/adc-lab-priv-helper"
[[ -x "$adc_lab" ]] || die "release tarball does not contain executable bin/adc-lab"
[[ -x "$adc_lab_target" ]] || die "release tarball does not contain executable bin/adc-lab-target"
[[ -f "$manifest" ]] || die "release tarball does not contain release-manifest.json"
if [[ -n "$version" ]]; then
  version_no_v="${version#v}"
  grep -F "\"version\": \"${version_no_v}\"" "$manifest" >/dev/null \
    || die "release manifest version does not match ${version}"
fi
grep -F '"name": "adc-lab-priv-helper"' "$manifest" >/dev/null \
  || die "release manifest does not list adc-lab-priv-helper"
grep -F '"name": "adc-lab"' "$manifest" >/dev/null \
  || die "release manifest does not list adc-lab"
grep -F '"name": "adc-lab-target"' "$manifest" >/dev/null \
  || die "release manifest does not list adc-lab-target"

resolved_version="$(grep -E '"version":' "$manifest" | head -n 1 | sed -E 's/.*"version": "([^"]+)".*/\1/')"
[[ -n "$resolved_version" ]] || die "failed to read release manifest version"

"$adc_lab" --version >/dev/null
"$adc_lab_target" --version >/dev/null
"$helper" --version >/dev/null

if [[ "$install_user_bins" == "true" ]]; then
  mkdir -p "$user_bin_dir"
  echo "Installing adc-lab user binaries to ${user_bin_dir}" >&2
  install -m 0755 "$adc_lab" "${user_bin_dir}/adc-lab"
  install -m 0755 "$adc_lab_target" "${user_bin_dir}/adc-lab-target"
  "${user_bin_dir}/adc-lab" --version >/dev/null
  "${user_bin_dir}/adc-lab-target" --version >/dev/null
fi

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

if [[ "$install_user_bins" == "true" ]]; then
  "${user_bin_dir}/adc-lab" privilege doctor \
    --target local \
    --run-dir "${tmp_dir}/privilege-doctor-run" \
    --json
else
  "$adc_lab" privilege doctor \
    --target local \
    --run-dir "${tmp_dir}/privilege-doctor-run" \
    --json
fi

echo "installed adc-lab release ${resolved_version}" >&2
