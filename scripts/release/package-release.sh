#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat >&2 <<'USAGE'
usage: scripts/release/package-release.sh \
  --version 0.1.0 \
  --git-sha <sha> \
  --target-triple <rust-target-triple> \
  --asset-triple <asset-triple> \
  --target-dir <cargo-target-dir> \
  --dist-dir <dist-dir>
USAGE
}

version=""
git_sha=""
target_triple=""
asset_triple=""
target_dir=""
dist_dir=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --version)
      version="${2:?missing --version value}"
      shift 2
      ;;
    --git-sha)
      git_sha="${2:?missing --git-sha value}"
      shift 2
      ;;
    --target-triple)
      target_triple="${2:?missing --target-triple value}"
      shift 2
      ;;
    --asset-triple)
      asset_triple="${2:?missing --asset-triple value}"
      shift 2
      ;;
    --target-dir)
      target_dir="${2:?missing --target-dir value}"
      shift 2
      ;;
    --dist-dir)
      dist_dir="${2:?missing --dist-dir value}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage
      exit 2
      ;;
  esac
done

for required in version git_sha target_triple asset_triple target_dir dist_dir; do
  if [[ -z "${!required}" ]]; then
    echo "missing required argument: --${required//_/-}" >&2
    usage
    exit 2
  fi
done

validate_token() {
  local label="$1"
  local value="$2"
  if [[ ! "$value" =~ ^[A-Za-z0-9._+-]+$ ]]; then
    echo "invalid ${label}: ${value}" >&2
    exit 2
  fi
}

validate_token "version" "$version"
validate_token "git sha" "$git_sha"
validate_token "target triple" "$target_triple"
validate_token "asset triple" "$asset_triple"

case "$target_triple" in
  aarch64-unknown-linux-gnu)
    expected_asset_triple="linux-aarch64"
    ;;
  x86_64-unknown-linux-gnu)
    expected_asset_triple="linux-x86_64"
    ;;
  *)
    echo "unsupported release target triple: $target_triple" >&2
    exit 2
    ;;
esac

if [[ "$asset_triple" != "$expected_asset_triple" ]]; then
  echo "asset triple ${asset_triple} does not match ${target_triple}; expected ${expected_asset_triple}" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
target_dir_abs="$(cd "$target_dir" && pwd)"
dist_dir_abs="$(mkdir -p "$dist_dir" && cd "$dist_dir" && pwd)"

asset_name="adc-lab-v${version}-${asset_triple}.tar.gz"
stage_parent="${dist_dir_abs}/stage"
stage_dir="${stage_parent}/adc-lab-v${version}-${asset_triple}"
rm -rf "$stage_dir"
mkdir -p "$stage_dir/bin"

copy_binary() {
  local name="$1"
  local source="${target_dir_abs}/${name}"
  local dest="${stage_dir}/bin/${name}"
  if [[ ! -f "$source" ]]; then
    echo "missing release binary: $source" >&2
    exit 1
  fi
  cp "$source" "$dest"
  chmod 0755 "$dest"
}

copy_required_file() {
  local source="$1"
  local dest="$2"
  if [[ ! -f "$repo_root/$source" ]]; then
    echo "missing release file: $source" >&2
    exit 1
  fi
  mkdir -p "$(dirname "$stage_dir/$dest")"
  cp "$repo_root/$source" "$stage_dir/$dest"
}

copy_binary "adc-lab"
copy_binary "adc-lab-target"
copy_binary "adc-lab-priv-helper"
copy_required_file "README.md" "README.md"
copy_required_file "LICENSE" "LICENSE"
copy_required_file "docs/getting-started/pi5-controller-pi4-target.md" \
  "docs/getting-started/pi5-controller-pi4-target.md"
copy_required_file "docs/getting-started/install-release-binaries.md" \
  "docs/getting-started/install-release-binaries.md"
copy_required_file "docs/getting-started/pi4-pi5-measurement-prompt.md" \
  "docs/getting-started/pi4-pi5-measurement-prompt.md"

sha_adc_lab="$(sha256sum "$stage_dir/bin/adc-lab" | awk '{print $1}')"
sha_adc_lab_target="$(sha256sum "$stage_dir/bin/adc-lab-target" | awk '{print $1}')"
sha_adc_lab_priv_helper="$(sha256sum "$stage_dir/bin/adc-lab-priv-helper" | awk '{print $1}')"

cat > "$stage_dir/release-manifest.json" <<JSON
{
  "schema_version": "lab.release_manifest.v1",
  "version": "${version}",
  "git_sha": "${git_sha}",
  "target_triple": "${target_triple}",
  "binaries": [
    {
      "name": "adc-lab",
      "sha256": "${sha_adc_lab}"
    },
    {
      "name": "adc-lab-target",
      "sha256": "${sha_adc_lab_target}"
    },
    {
      "name": "adc-lab-priv-helper",
      "sha256": "${sha_adc_lab_priv_helper}"
    }
  ]
}
JSON

tar -czf "${dist_dir_abs}/${asset_name}" -C "$stage_dir" .
echo "${dist_dir_abs}/${asset_name}"
