# 0003 WebView plugin UI (JUCE 8)

- Status: Accepted
- Date: 2026-11-01

## Context

Native JUCE widgets are fine for skeletons, but authoring comfort and iteration speed favor web UI. JUCE 8 provides `WebBrowserComponent`, native integration, parameter relays/attachments, and a resource provider suitable for packaging static frontends.

## Decision

- Use **JUCE 8 WebView** as the plugin UI strategy for Prism (and as the default pattern for future UI-heavy plugins).
- Frontend stack: **Vite + React + TypeScript**, with JUCE’s `juce-framework-frontend` JS package vendored under each plugin’s `ui/` tree so npm install does not require a prior CMake configure.
- Production: serve built assets via `juce_add_binary_data` + `withResourceProvider`.
- Development: optional Vite localhost URL (`http://localhost:5173`) with resource-provider origin allowlisting.
- CMake: `dsp_add_plugin(... WEB_UI TRUE)` enables `JUCE_WEB_BROWSER=1`, links `juce_gui_extra`, and sets keyboard focus for the editor.

## Consequences

- Node.js is required to build UI assets before packaging BinaryData.
- Plugin binaries gain WebKit (macOS) / WebView2 (Windows) dependencies.
- Parameter wiring uses `WebSliderRelay` / combo / toggle relays + APVTS attachments rather than custom native controls.
- Curl remains disabled (`JUCE_USE_CURL=0`).
