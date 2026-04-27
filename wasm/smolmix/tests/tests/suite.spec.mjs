// Full test suite: runs the headless test runner for both traffic configs.
//
// Each config gets its own page load (OnceLock prevents tunnel re-init).
// The headless.js runner auto-executes: smoke, HTTPS warm, stress httpbin,
// stress drip — then outputs RESULTS_JSON for parsing.
//
// Env: IPR_ADDRESS (optional, uses default if omitted)

import { test, expect } from "@playwright/test";

const BASE_URL = "http://localhost:9001/headless.html";
const IPR_ADDRESS = process.env.IPR_ADDRESS;

const CONFIGS = [
  {
    name: "no cover, no Poisson",
    params: "count=5",
  },
  {
    name: "with cover + Poisson",
    params: "cover=true&poisson=true&count=5",
  },
];

function waitForConsole(page, predicate, timeoutMs) {
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

for (const cfg of CONFIGS) {
  test.describe(cfg.name, () => {
    test("full suite", async ({ page }) => {
      // Forward all logs to test output for debugging.
      page.on("console", (msg) => {
        const text = msg.text();
        if (
          text.startsWith("[") ||
          text.includes("===") ||
          text.includes("Config:") ||
          text.includes("FATAL") ||
          text.includes("RESULTS_JSON")
        ) {
          console.log(text);
        }
      });

      page.on("pageerror", (err) => {
        console.log(`[PAGEERROR] ${err.message}`);
      });

      // Build URL with config params + optional IPR override.
      let url = `${BASE_URL}?${cfg.params}`;
      if (IPR_ADDRESS) {
        url += `&ipr=${encodeURIComponent(IPR_ADDRESS)}`;
      }

      // Wait for the RESULTS_JSON console message (the suite auto-runs).
      const jsonPromise = waitForConsole(
        page,
        (text) => text.startsWith("RESULTS_JSON:"),
        540_000 // 9 minutes — generous for mixnet latency
      );

      await page.goto(url);
      const resultLine = await jsonPromise;
      const json = JSON.parse(resultLine.replace("RESULTS_JSON:", ""));

      // --- Timing summary ---
      console.log("");
      console.log("================================================================");
      console.log(`  Config: ${cfg.name}`);
      console.log(`  Date:   ${json.date}`);
      console.log("================================================================");
      console.log("");
      console.log(
        `  ${"Test".padEnd(28)}${"Result".padEnd(10)}${"Time".padEnd(10)}Details`
      );
      console.log(`  ${"".padEnd(28, "-")}${"".padEnd(10, "-")}${"".padEnd(10, "-")}${"".padEnd(20, "-")}`);

      for (const r of json.results) {
        let resultStr =
          r.total !== undefined ? `${r.okCount}/${r.total}` : r.ok ? "PASS" : "FAIL";
        let timeStr = r.ms ? `${(r.ms / 1000).toFixed(2)}s` : "N/A";
        let details = "";
        if (r.avgMs !== undefined) {
          details = `avg ${(r.avgMs / 1000).toFixed(2)}s/req`;
        }
        if (r.error) {
          details = r.error.slice(0, 60);
        }
        console.log(
          `  ${r.name.padEnd(28)}${resultStr.padEnd(10)}${timeStr.padEnd(10)}${details}`
        );
      }

      console.log("");
      console.log("================================================================");

      // --- Assertions ---

      // Check for fatal setup errors first.
      const fatal = json.results.find((r) => r.error && !r.ok && r.ms === 0);
      if (fatal) {
        expect.soft(false, `Fatal error: ${fatal.error}`).toBeTruthy();
        return;
      }

      // Smoke must pass.
      const smoke = json.results.find((r) => r.name.includes("Smoke"));
      expect(smoke, "Smoke test result should exist").toBeTruthy();
      expect(smoke.ok, "Smoke test (cold HTTPS GET) should pass").toBeTruthy();

      // HTTPS warm must pass.
      const warm = json.results.find((r) => r.name.includes("warm"));
      expect(warm, "HTTPS warm result should exist").toBeTruthy();
      expect(warm.ok, "HTTPS GET (warm) should pass").toBeTruthy();

      // Warm should be significantly faster than cold.
      if (smoke.ok && warm.ok) {
        console.log(
          `  Cold vs warm: ${(smoke.ms / 1000).toFixed(1)}s -> ${(warm.ms / 1000).toFixed(1)}s ` +
            `(${(smoke.ms / warm.ms).toFixed(1)}x speedup from connection pooling)`
        );
      }

      // Stress httpbin: >= 80% success.
      const stress = json.results.find((r) => r.name.includes("httpbin"));
      if (stress?.total) {
        const rate = stress.okCount / stress.total;
        console.log(
          `  Stress httpbin success rate: ${(rate * 100).toFixed(0)}%`
        );
        expect(rate).toBeGreaterThanOrEqual(0.8);
      }

    });
  });
}
