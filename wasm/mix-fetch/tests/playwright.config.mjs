import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 60_000,
  retries: 1,
  use: {
    trace: "on-first-retry",
  },
  webServer: {
    command: "npx serve ../internal-dev/dist -l 8001 --no-clipboard",
    port: 8001,
    reuseExistingServer: true,
  },
  projects: [
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
    {
      name: "stress-chromium",
      testMatch: "stress.spec.mjs",
      timeout: 120_000,
      retries: 2,
      use: { browserName: "chromium" },
    },
    {
      name: "stress-firefox",
      testMatch: "stress.spec.mjs",
      timeout: 120_000,
      retries: 2,
      use: { browserName: "firefox" },
    },
    {
      name: "stress-webkit",
      testMatch: "stress.spec.mjs",
      timeout: 120_000,
      retries: 2,
      use: { browserName: "webkit" },
    },
  ],
});
