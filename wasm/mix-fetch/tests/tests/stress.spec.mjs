// Stress test: connect to mainnet via a random Entry Gateway and run concurrent fetches.
// Pass criteria: >= 80% of requests succeed.

import { test, expect } from "@playwright/test";

const STRESS_COUNT = 10;
const MIN_SUCCESS_RATE = 0.8;

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

test("stress test: mixed-size fetches through mainnet", async ({ page }) => {
  // Forward warnings, errors, and worker lifecycle messages to test output
  page.on("console", (msg) => {
    const text = msg.text();
    if (msg.type() === "warning" || msg.type() === "error") {
      console.log(`[${msg.type().toUpperCase()}] ${text}`);
    } else if (
      text.includes("Worker") ||
      text.includes("MixFetch") ||
      text.includes("stress") ||
      text.includes("COMPLETE") ||
      text.includes("FAIL")
    ) {
      console.log(text);
    }
  });

  const workerReady = waitForConsole(
    page,
    (text) => text.includes("Worker ready"),
    30_000
  );
  await page.goto("http://localhost:8001");
  await workerReady;

  const mixFetchReady = waitForConsole(
    page,
    (text) => text.includes("MixFetch ready!"),
    120_000
  );
  await page.check('input[name="gateway-mode"][value="random"]');
  await page.click("#start-mixfetch");
  await mixFetchReady;

  const stressComplete = waitForConsole(
    page,
    (text) => text.includes("=== COMPLETE:"),
    90_000
  );
  await page.fill("#stress-test-count", String(STRESS_COUNT));
  await page.selectOption("#stress-test-mode", "mixed");
  await page.click("#stress-test-button");
  const completionMsg = await stressComplete;

  const match = completionMsg.match(/OK (\d+)\/(\d+)/);
  expect(
    match,
    `Could not parse completion message: ${completionMsg}`
  ).toBeTruthy();

  const [, succeeded, total] = match;
  const successRate = parseInt(succeeded, 10) / parseInt(total, 10);

  console.log(
    `Stress test result: ${succeeded}/${total} succeeded (${(
      successRate * 100
    ).toFixed(0)}%)`
  );
  expect(successRate).toBeGreaterThanOrEqual(MIN_SUCCESS_RATE);
});
