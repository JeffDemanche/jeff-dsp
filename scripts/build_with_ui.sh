#!/usr/bin/env bash
# Build WebView UI assets (when present), then the JUCE/Rust plugin(s).
# Usage:
#   ./scripts/build_with_ui.sh              # all plugins, Release
#   ./scripts/build_with_ui.sh prism        # one plugin, Release
#   ./scripts/build_with_ui.sh prism Debug  # one plugin, Debug
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

PLUGIN="${1:-}"
CONFIG="${2:-Release}"

build_ui_for() {
  local plugin_dir="$1"
  local ui_dir="${plugin_dir}/ui"
  if [[ ! -f "${ui_dir}/package.json" ]]; then
    return 0
  fi

  local name
  name="$(basename "${plugin_dir}")"
  echo "==> Building UI (${name})"
  if [[ ! -d "${ui_dir}/node_modules" ]]; then
    (cd "${ui_dir}" && npm install)
  fi
  (cd "${ui_dir}" && npm run build)
}

if [[ -n "${PLUGIN}" ]]; then
  PLUGIN_DIR="${ROOT}/plugins/${PLUGIN}"
  if [[ ! -d "${PLUGIN_DIR}" ]]; then
    echo "Unknown plugin '${PLUGIN}' (expected plugins/${PLUGIN})" >&2
    exit 1
  fi
  build_ui_for "${PLUGIN_DIR}"
else
  for plugin_dir in "${ROOT}"/plugins/*/; do
    [[ -d "${plugin_dir}" ]] || continue
    build_ui_for "${plugin_dir}"
  done
fi

exec "${ROOT}/scripts/build.sh" ${PLUGIN:+"${PLUGIN}"} "${CONFIG}"
