// Takes the screenshots in docs/images from the real UI with the fake
// backend (e2e/fakeBackend.ts, replaying real Apio output).
// Run: npm run docs:screenshots
import { defineConfig, devices } from "@playwright/test";
import base from "./playwright.config";

export default defineConfig({
  ...base,
  testMatch: "**/*.shots.ts",
  fullyParallel: false,
  reporter: "list",
  projects: [
    {
      name: "screenshots",
      use: {
        ...devices["Desktop Safari"],
        // About the size of the Logix window, at 2x for Retina screens.
        viewport: { width: 1100, height: 640 },
        deviceScaleFactor: 2,
        colorScheme: "light",
      },
    },
  ],
});
