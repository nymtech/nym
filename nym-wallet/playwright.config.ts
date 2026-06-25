import { defineConfig, devices } from '@playwright/test';

/**
 * Primary e2e config (design D1/D10). Drives the Family page inside the real app shell +
 * router via the mock-wired dev server (`main.mock.html`, design D2) — a real browser
 * session with no Tauri runtime or chain. Cross-platform, runs locally on macOS.
 *
 * Requires the workspace packages to be built (`pnpm --dir .. run build`) plus
 * `npx playwright install chromium`. The webServer launches `webpack:dev:mock` on :9000
 * with `WALLET_MOCK_FAMILIES=on` (reused if already running locally).
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: 'list',
  // Build the static visual flow report (e2e-report/) from per-step screenshots.
  globalSetup: './e2e/report.globalSetup.ts',
  globalTeardown: './e2e/report.globalTeardown.ts',
  timeout: 60_000,
  use: {
    baseURL: 'http://localhost:9000',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'npm run webpack:dev:mock',
    // The mock entry's generated page — only exists when WALLET_MOCK_FAMILIES=on.
    url: 'http://localhost:9000/main.mock.html',
    reuseExistingServer: !process.env.CI,
    timeout: 300_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
