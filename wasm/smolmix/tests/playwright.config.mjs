import { defineConfig } from "@playwright/test";

// Headless Firefox on non-Ubuntu hosts (Arch/Manjaro fallback build) needs
// these prefs to behave like headed Firefox: keep IndexedDB persistent across
// the test session, raise the wss:// open timeout above the 20 s default
// (mixnet handshake + WSS upgrade can take longer on a cold load), and skip
// the captive-portal probe that adds spurious DNS noise.
const firefoxUserPrefs = {
  "dom.indexedDB.enabled": true,
  "dom.storage.next_gen": true,
  "browser.privatebrowsing.autostart": false,
  "network.websocket.timeout.open": 60,
  "network.captive-portal-service.enabled": false,
  "network.connectivity-service.enabled": false,
  // Force the OS resolver rather than Mozilla's DoH. Headless Firefox on
  // non-Ubuntu hosts sometimes can't reach mozilla.cloudflare-dns.com from
  // the bundled NSS context, which stalls gateway hostname resolution.
  "network.trr.mode": 0,
  "network.dns.disablePrefetch": true,
  // Headless Firefox has no visible window so it treats itself as backgrounded
  // and clamps timers (default 1 s minimum via dom.min_background_timeout_value).
  // Our reactor ticks every 5 ms and the Nym base client uses many short
  // timers in its WS keepalive/retry logic; clamping those to 1 s stalls the
  // gateway handshake. Disable every timer-throttling knob.
  "dom.min_background_timeout_value": 4,
  "dom.workers.timeoutThrottling": false,
  "dom.timeout.foreground_budget_regeneration_rate": -1,
  "dom.timeout.background_budget_regeneration_rate": -1,
};

const firefoxUse = {
  browserName: "firefox",
  launchOptions: { firefoxUserPrefs },
};

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
      use: firefoxUse,
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
      use: firefoxUse,
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
