import { defineConfig, devices } from '@playwright/test';

/**
 * Playwright e2e config (tasks.md §8.3). The production app is Tauri, so e2e runs
 * against the Storybook flow stories served as a real browser session (design D8).
 *
 * Requires `pnpm install` (adds @playwright/test) + `npx playwright install chromium`.
 * The webServer serves Storybook on :6006 (reused if already running).
 */
export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: 'list',
  timeout: 60_000,
  use: {
    baseURL: 'http://localhost:6006',
    trace: 'on-first-retry',
  },
  webServer: {
    command: 'npm run storybook -- --ci -p 6006',
    url: 'http://localhost:6006',
    reuseExistingServer: !process.env.CI,
    timeout: 180_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
});
