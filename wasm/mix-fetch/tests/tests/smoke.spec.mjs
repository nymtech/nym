// Smoke test: verify the internal-dev harness loads both WASM runtimes
// and successfully initialises a MixFetch connection to mainnet.

import { test, expect } from "@playwright/test";

function waitForConsole(page, predicate, timeoutMs = 60_000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(
      () =>
        reject(
          new Error(`Timed out waiting for console message (${timeoutMs}ms)`)
        ),
      timeoutMs
    );
    page.on("console", function handler(msg) {
      if (predicate(msg.text())) {
        clearTimeout(timer);
        page.removeListener("console", handler);
        resolve(msg.text());
      }
    });
  });
}

test("internal-dev harness loads and MixFetch initialises", async ({
  page,
}) => {
  const errors = [];

  // Forward worker lifecycle + errors to test output
  page.on("console", (msg) => {
    const text = msg.text();
    if (msg.type() === "error") {
      if (!text.includes("favicon.ico")) {
        errors.push(text);
      }
      console.log(`[ERROR] ${text}`);
    } else if (
      text.includes("Worker") ||
      text.includes("MixFetch") ||
      text.includes("Setting up") ||
      text.includes("gateway")
    ) {
      console.log(text);
    }
  });

  page.on("pageerror", (err) => {
    errors.push(`pageerror: ${err.message}`);
  });

  const workerReady = waitForConsole(
    page,
    (text) => text.includes("Worker ready"),
    30_000
  );
  await page.goto("http://localhost:8001");
  await workerReady;

  // Init MixFetch with a random gateway
  const mixFetchReady = waitForConsole(
    page,
    (text) => text.includes("MixFetch ready!"),
    120_000
  );
  await page.check('input[name="gateway-mode"][value="random"]');
  await page.click("#start-mixfetch");
  await mixFetchReady;

  expect(errors).toEqual([]);
});
