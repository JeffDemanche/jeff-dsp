import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

/**
 * JUCE resource-provider WebViews (juce://) reject ES modules / CORS.
 * Serve a single classic IIFE, with the script at end of <body> (no defer) —
 * deferred scripts from custom URL schemes are unreliable in WKWebView.
 */
function classicScriptsForJuce(): Plugin {
  return {
    name: "classic-scripts-for-juce",
    enforce: "post",
    transformIndexHtml(html) {
      const withoutModuleAttrs = html
        .replace(/\s+type="module"/g, "")
        .replace(/\s+crossorigin(?:="[^"]*")?/g, "")
        .replace(/\s+defer\b/g, "");

      // Move any <script src=...> out of <head> to end of <body>
      const scripts: string[] = [];
      const htmlNoScripts = withoutModuleAttrs.replace(
        /<script\b[^>]*\bsrc=(["'][^"']+["'])[^>]*><\/script>/gi,
        (tag) => {
          scripts.push(tag.replace(/\bsrc=(["'])\.?\//, "src=$1/"));
          return "";
        }
      );

      if (scripts.length === 0) return withoutModuleAttrs;

      return htmlNoScripts.replace(
        /<\/body>/i,
        `${scripts.join("\n    ")}\n  </body>`
      );
    },
  };
}

export default defineConfig({
  plugins: [react(), classicScriptsForJuce()],
  base: "./",
  build: {
    outDir: "dist",
    emptyOutDir: true,
    cssCodeSplit: false,
    modulePreload: false,
    assetsDir: ".",
    rollupOptions: {
      output: {
        format: "iife",
        name: "PrismUI",
        inlineDynamicImports: true,
        entryFileNames: "app.js",
        assetFileNames: "[name][extname]",
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
