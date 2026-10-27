import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig(({ mode }) => {
  // "e2e": the UI runs in a plain browser for the Playwright tests, with a
  // fake backend instead of Tauri and Apio (see e2e/fakeBackend.ts).
  const e2e = mode === "e2e";
  return {
    // The UI project is this folder (ui/); npm runs from the repo root.
    root: fileURLToPath(new URL(".", import.meta.url)),
    plugins: [svelte()],
    clearScreen: false,
    resolve: {
      alias: e2e
        ? [
            {
              // "./lib/backend" from App.svelte, "./backend" from src/lib.
              find: /^\.\/(lib\/)?backend$/,
              replacement: fileURLToPath(new URL("./e2e/fakeBackend.ts", import.meta.url)),
            },
          ]
        : [],
    },
    server: {
      // Tauri expects a fixed dev server port (see src-tauri/tauri.conf.json).
      // The e2e tests use another one, so they can run next to ./run.sh.
      port: e2e ? 1430 : 1420,
      strictPort: true,
      watch: { ignored: ["**/src-tauri/**"] },
    },
  };
});
