// Smoke test: verify the internal-dev harness loads WASM and connects
// to the mixnet via an IPR.
//
// Env: IPR_ADDRESS (optional). Without it, the page's pre-filled default
// (internal-dev/index.html `#ipr-address` value attribute) is used.

import { test, expect } from "@playwright/test";

const IPR_ADDRESS = process.env.IPR_ADDRESS;

function waitForConsole(page, predicate, timeoutMs = 120_000) {
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

test("WASM loads and tunnel connects to IPR", async ({ page }) => {
  const errors = [];

  page.on("console", (msg) => {
    const text = msg.text();
    if (text.startsWith("[")) {
      console.log(text);
    }
    if (msg.type() === "error" && !text.includes("favicon.ico")) {
      errors.push(text);
      console.log(`[ERROR] ${text}`);
    }
  });

  page.on("pageerror", (err) => {
    errors.push(`pageerror: ${err.message}`);
  });

  await page.goto("http://localhost:9001");
  await page.waitForSelector("#btn-setup");

  // The input is pre-filled with a working default; only override when the
  // caller passed an explicit IPR_ADDRESS env var.
  if (IPR_ADDRESS) {
    await page.fill("#ipr-address", IPR_ADDRESS);
  }

  // Race: tunnel ready OR fatal error — whichever comes first.
  const tunnelReady = waitForConsole(
    page,
    (text) =>
      text.includes("setupMixTunnel OK") || text.includes("tunnel ready"),
    120_000
  );
  const fatalError = waitForConsole(
    page,
    (text) => text.includes("FATAL") || text.includes("tunnel error"),
    120_000
  );
  await page.click("#btn-setup");

  const result = await Promise.race([
    tunnelReady.then((msg) => ({ ok: true, msg })),
    fatalError.then((msg) => ({ ok: false, msg })),
  ]);
  expect(result.ok, `Tunnel setup failed: ${result.msg}`).toBeTruthy();

  const hardErrors = errors.filter(
    (e) => !e.includes("favicon") && !e.includes("DevTools")
  );
  expect(hardErrors).toEqual([]);
});
