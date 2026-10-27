// End-to-end UI tests: the real UI in WebKit (the engine behind the macOS
// app window) with a fake backend (e2e/fakeBackend.ts). Run: npm run test:e2e
import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "e2e",
  testMatch: "**/*.e2e.ts",
  // Next to this config (ui/test-results), not where npm runs.
  outputDir: "test-results",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://localhost:1430",
    trace: "retain-on-failure",
  },
  projects: [{ name: "webkit", use: { ...devices["Desktop Safari"] } }],
  webServer: {
    // Vite in "e2e" mode swaps in the fake backend.
    command: "npm run dev -- --mode e2e",
    // npm and the scripts run from the repo root.
    cwd: "..",
    url: "http://localhost:1430",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
