#!/usr/bin/env bash
# One-line bootstrap installer for Token 戰情室 (Linux / macOS).
#
# Downloads the correct prebuilt release archive from GitHub Releases (no
# Rust/Cargo toolchain required), extracts it, and runs the packaged
# install.sh. Safe to re-run to upgrade to a newer release.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/fun-ed/TokenUsageInsights/main/scripts/get.sh | bash
#   curl -fsSL .../get.sh | bash -s -- --service
#
# Environment:
#   TOKEN_USAGE_INSIGHTS_VERSION       Release tag to install. Default: latest
#   TOKEN_USAGE_INSIGHTS_INSTALL_DIR   Forwarded to install.sh
#   TOKEN_USAGE_INSIGHTS_BIN_DIR       Forwarded to install.sh
set -euo pipefail

repo="fun-ed/TokenUsageInsights"
app_name="token-usage-insights"
version="${TOKEN_USAGE_INSIGHTS_VERSION:-latest}"

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)
    if [[ "$arch" != "x86_64" && "$arch" != "amd64" ]]; then
      echo "Unsupported Linux architecture: ${arch} (only x86_64 builds are published)" >&2
      exit 1
    fi
    target="x86_64-unknown-linux-gnu"
    archive_ext="tar.gz"
    ;;
  Darwin)
    case "$arch" in
      arm64) target="aarch64-apple-darwin" ;;
      x86_64) target="x86_64-apple-darwin" ;;
      *)
        echo "Unsupported macOS architecture: ${arch}" >&2
        exit 1
        ;;
    esac
    archive_ext="tar.gz"
    ;;
  *)
    echo "Unsupported OS: ${os}. This installer supports Linux and macOS." >&2
    exit 1
    ;;
esac

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required but was not found." >&2
  exit 1
fi

if [[ "$version" == "latest" ]]; then
  echo "Resolving latest release tag for ${repo} ..."
  # Follow the /releases/latest redirect to /releases/tag/<tag> and take the
  # final path component. This avoids piping curl into grep/sed, which races
  # with `set -o pipefail` and intermittently fails with curl: (23) SIGPIPE
  # when grep -m1 exits before curl has finished writing the API response.
  latest_url="$(curl -fsSL -o /dev/null -w '%{url_effective}' \
    "https://github.com/${repo}/releases/latest")"
  tag="${latest_url##*/}"
  if [[ -z "$tag" || "$tag" == "latest" ]]; then
    echo "Failed to resolve the latest release tag." >&2
    exit 1
  fi
else
  tag="$version"
fi

archive="${app_name}-${tag}-${target}.${archive_ext}"
url="https://github.com/${repo}/releases/download/${tag}/${archive}"

workdir="$(mktemp -d)"
trap 'rm -rf "$workdir"' EXIT

echo "Downloading ${url} ..."
if ! curl -fsSL "$url" -o "${workdir}/${archive}"; then
  echo "Download failed. Check that ${tag} publishes a ${target} archive:" >&2
  echo "  https://github.com/${repo}/releases/tag/${tag}" >&2
  exit 1
fi

echo "Extracting ..."
tar -xzf "${workdir}/${archive}" -C "$workdir"

extracted_dir="${workdir}/${app_name}-${tag}-${target}"
if [[ ! -d "$extracted_dir" ]]; then
  extracted_dir="$(find "$workdir" -mindepth 1 -maxdepth 1 -type d | head -n1)"
fi

install_script="${extracted_dir}/scripts/install.sh"
if [[ ! -f "$install_script" ]]; then
  install_script="${extracted_dir}/install.sh"
fi
if [[ ! -f "$install_script" ]]; then
  echo "Installer not found in ${archive}." >&2
  exit 1
fi

echo "Installing ${tag} ..."
bash "$install_script" "$@"
