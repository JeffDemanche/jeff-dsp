#!/usr/bin/env bash
# Build one or all plugins.
# Usage:
#   ./scripts/build.sh              # all plugins, Release
#   ./scripts/build.sh prism        # one plugin, Release
#   ./scripts/build.sh prism Debug  # one plugin, Debug
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

PLUGIN="${1:-}"
CONFIG="${2:-Release}"

case "$(echo "${CONFIG}" | tr '[:upper:]' '[:lower:]')" in
  debug) PRESET="debug" ;;
  release) PRESET="release" ;;
  *)
    echo "Unknown config '${CONFIG}' (use Debug or Release)" >&2
    exit 1
    ;;
esac

CONFIGURE_ARGS=(--preset "${PRESET}")
if [[ -n "${PLUGIN}" ]]; then
  CONFIGURE_ARGS+=(-DDSP_PLUGINS="${PLUGIN}")
fi

echo "==> Configuring (${PRESET})${PLUGIN:+, plugin=${PLUGIN}}"
cmake "${CONFIGURE_ARGS[@]}"

# With DSP_PLUGINS set, building the default target builds every format for
# the selected plugin(s). Avoid targeting only the shared-code library.
echo "==> Building"
cmake --build --preset "${PRESET}" --parallel

echo "==> Done. Artefacts under build/${PRESET}/"
