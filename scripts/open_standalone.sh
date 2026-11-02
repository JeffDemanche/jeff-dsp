#!/usr/bin/env bash
# Open a built Standalone plugin app (macOS).
# Usage:
#   ./scripts/open_standalone.sh              # prism, Release
#   ./scripts/open_standalone.sh prism        # Release
#   ./scripts/open_standalone.sh prism Debug
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

PLUGIN="${1:-prism}"
CONFIG="${2:-Release}"

case "$(echo "${CONFIG}" | tr '[:upper:]' '[:lower:]')" in
  debug)
    PRESET="debug"
    CONFIG_DIR="Debug"
    ;;
  release)
    PRESET="release"
    CONFIG_DIR="Release"
    ;;
  *)
    echo "Unknown config '${CONFIG}' (use Debug or Release)" >&2
    exit 1
    ;;
esac

PLUGIN_BUILD_DIR="${ROOT}/build/${PRESET}/plugins/${PLUGIN}"
if [[ ! -d "${PLUGIN_BUILD_DIR}" ]]; then
  echo "No build at ${PLUGIN_BUILD_DIR}" >&2
  echo "Build first: ./scripts/build_with_ui.sh ${PLUGIN} ${CONFIG}" >&2
  exit 1
fi

# Artefacts live under <Target>_artefacts/<Config>/Standalone/<Product>.app
shopt -s nullglob
apps=( "${PLUGIN_BUILD_DIR}"/*_artefacts/"${CONFIG_DIR}"/Standalone/*.app )
shopt -u nullglob

if [[ ${#apps[@]} -eq 0 ]]; then
  echo "No Standalone .app found under ${PLUGIN_BUILD_DIR}/*_artefacts/${CONFIG_DIR}/Standalone/" >&2
  echo "Build first: ./scripts/build_with_ui.sh ${PLUGIN} ${CONFIG}" >&2
  exit 1
fi

APP="${apps[0]}"
if [[ ${#apps[@]} -gt 1 ]]; then
  echo "Multiple Standalone apps found; opening the first:" >&2
  printf '  %s\n' "${apps[@]}" >&2
fi

PRODUCT_NAME="$(basename "${APP}" .app)"
BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "${APP}/Contents/Info.plist" 2>/dev/null || true)"
if [[ -z "${BUNDLE_ID}" ]]; then
  BUNDLE_ID="com.jeffdsp.$(echo "${PLUGIN}" | tr '[:upper:]' '[:lower:]')"
fi

# Ad-hoc re-sign with a stable identifier so macOS TCC can show the app under
# Privacy → Microphone (linker-signed builds often never appear in the list).
if [[ "$(uname -s)" == "Darwin" ]]; then
  echo "==> codesign (adhoc) ${BUNDLE_ID}"
  codesign --force --deep --sign - --identifier "${BUNDLE_ID}" "${APP}" 2>&1 || true
fi

# JUCE Standalone mutes input by default (feedback protection). Pref-write unmute
# so loop-through monitoring works when launched from this script.
# Path matches StandaloneFilterApp PropertiesFile options for product name.
SETTINGS_FILE="${HOME}/Library/Application Support/${PRODUCT_NAME}.settings"
mkdir -p "$(dirname "${SETTINGS_FILE}")"
python3 - "${SETTINGS_FILE}" <<'PY'
import pathlib, sys, xml.etree.ElementTree as ET

path = pathlib.Path(sys.argv[1])
root = None
if path.exists():
    try:
        root = ET.parse(path).getroot()
    except ET.ParseError:
        root = None

if root is None or root.tag != "PROPERTIES":
    root = ET.Element("PROPERTIES")

found = False
for child in list(root):
    if child.tag == "VALUE" and child.attrib.get("name") == "shouldMuteInput":
        child.attrib["val"] = "0"
        found = True
        break

if not found:
    ET.SubElement(root, "VALUE", {"name": "shouldMuteInput", "val": "0"})

path.write_text(ET.tostring(root, encoding="unicode") + "\n", encoding="utf-8")
print(f"==> Ensured shouldMuteInput=0 in {path}")
PY

echo "==> Opening ${APP}"
echo "    If macOS asks for microphone access, click Allow."
echo "    If still silent: Options → Audio/MIDI Settings → uncheck Mute audio input"
echo "    Privacy list: System Settings → Privacy & Security → Microphone → enable ${PRODUCT_NAME}"
open "${APP}"
