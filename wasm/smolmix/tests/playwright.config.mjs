import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 180_000,
  retries: 1,
  use: {
    trace: "on-first-retry",
  },
  webServer: {
    command: "npx serve ../internal-dev/dist -l 9001 --no-clipboard",
    port: 9001,
    reuseExistingServer: true,
  },
  projects: [
    // Smoke: all three browsers
    {
      name: "smoke-chromium",
      testMatch: "smoke.spec.mjs",
      use: { browserName: "chromium" },
    },
    {
      name: "smoke-firefox",
      testMatch: "smoke.spec.mjs",
      use: { browserName: "firefox" },
    },
    {
      name: "smoke-webkit",
      testMatch: "smoke.spec.mjs",
      use: { browserName: "webkit" },
    },
    // Suite: all three browsers
    {
      name: "suite-chromium",
      testMatch: "suite.spec.mjs",
      timeout: 600_000,
      retries: 0,
      use: { browserName: "chromium" },
    },
    {
      name: "suite-firefox",
      testMatch: "suite.spec.mjs",
      timeout: 600_000,
      retries: 0,
      use: { browserName: "firefox" },
    },
    {
      name: "suite-webkit",
      testMatch: "suite.spec.mjs",
      timeout: 600_000,
      retries: 0,
      use: { browserName: "webkit" },
    },
  ],
});
