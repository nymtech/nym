// smolmix-wasm headless test runner
//
// Auto-runs a battery of tests on page load. Config via URL params:
//
//   ?ipr=<address>     IPR address (uses default if omitted)
//   ?cover=true        Enable cover traffic (default: disabled)
//   ?poisson=true      Enable Poisson traffic (default: disabled)
//   ?count=10          Stress test request count
//
// Two runs needed for the full matrix (OnceLock prevents re-init):
//   http://localhost:9000/headless.html
//   http://localhost:9000/headless.html?cover=true&poisson=true

import * as Comlink from "comlink";

// Config

const params = new URLSearchParams(location.search);

const IPR_ADDRESS =
  params.get("ipr") ||
  "6B6iuWX4bQP4GVA4Yq7XmZencaaGw6BaPY6xJWYSwsbF.6g6LRx1fgU2Q2A4ZPKonYHtfBARh1GPMe1LtXk6vpRR8@q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1";

const ENABLE_COVER = params.get("cover") === "true";
const ENABLE_POISSON = params.get("poisson") === "true";
const STRESS_COUNT = parseInt(params.get("count") || "10", 10);

const CONFIG_LABEL = `cover=${ENABLE_COVER ? "ON" : "OFF"}, poisson=${
  ENABLE_POISSON ? "ON" : "OFF"
}`;

// Output

const outputEl = document.getElementById("output");

function log(msg) {
  const ts = new Date().toISOString().slice(11, 23);
  const line = `[${ts}] ${msg}`;
  outputEl.textContent += line + "\n";
  outputEl.scrollTop = outputEl.scrollHeight;
  console.log(line);
}

// Worker setup (shared worker.js)

function createWorker() {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL("./worker.js", import.meta.url));
    worker.addEventListener("error", reject);
    worker.addEventListener(
      "message",
      (msg) => {
        if (msg.data?.kind === "Loaded") {
          worker.removeEventListener("error", reject);
          resolve(Comlink.wrap(worker));
        }
      },
      { once: true }
    );
  });
}

function toResponse(raw) {
  return new Response(raw.body, {
    status: raw.status,
    statusText: raw.statusText,
    headers: new Headers(raw.headers),
  });
}

// Helpers

async function timedFetch(api, url, init = {}) {
  const t0 = performance.now();
  const raw = await api.mixFetch(url, init);
  const ms = performance.now() - t0;
  const resp = toResponse(raw);
  return { ok: resp.ok, status: resp.status, statusText: resp.statusText, ms };
}

function fmtMs(ms) {
  return (ms / 1000).toFixed(2) + "s";
}

// Test: Smoke (cold HTTPS GET)

async function testSmoke(api) {
  log("");
  log("=== Smoke Test (cold HTTPS GET) ===");
  log("GET https://httpbin.org/get");

  try {
    const r = await timedFetch(api, "https://httpbin.org/get");
    log(`  ${r.status} ${r.statusText} in ${fmtMs(r.ms)}`);
    return { name: "Smoke (cold HTTPS)", ok: true, ms: r.ms };
  } catch (e) {
    log(`  FAIL: ${e}`);
    return { name: "Smoke (cold HTTPS)", ok: false, ms: 0, error: String(e) };
  }
}

// Test: HTTPS GET (warm, pooled connection)

async function testHttpsGetWarm(api) {
  log("");
  log("=== HTTPS GET (warm) ===");
  log("GET https://httpbin.org/get (pooled connection)");

  try {
    const r = await timedFetch(api, "https://httpbin.org/get");
    log(`  ${r.status} ${r.statusText} in ${fmtMs(r.ms)}`);
    return { name: "HTTPS GET (warm)", ok: true, ms: r.ms };
  } catch (e) {
    log(`  FAIL: ${e}`);
    return { name: "HTTPS GET (warm)", ok: false, ms: 0, error: String(e) };
  }
}

// Test: Stress (httpbin mixed sizes)

const SIZE_PROFILES = [
  { label: "tiny", bytes: 128 },
  { label: "small", bytes: 1024 },
  { label: "medium", bytes: 10240 },
  { label: "large", bytes: 102400 },
];

async function testStressHttpbin(api) {
  log("");
  log(`=== Stress Test: httpbin (${STRESS_COUNT} requests, mixed sizes) ===`);

  const requests = [];
  for (let i = 0; i < STRESS_COUNT; i++) {
    const p = SIZE_PROFILES[Math.floor(Math.random() * SIZE_PROFILES.length)];
    requests.push({
      id: i + 1,
      url: `https://httpbin.org/bytes/${p.bytes}`,
      label: p.label,
    });
  }

  // Log profile distribution
  const dist = {};
  for (const r of requests) dist[r.label] = (dist[r.label] || 0) + 1;
  log(`  Profiles: ${JSON.stringify(dist)}`);

  const t0 = performance.now();
  const perReq = [];

  // Fire all concurrently (origin lock serialises per-host)
  const settled = await Promise.allSettled(
    requests.map(async (req) => {
      const start = performance.now();
      try {
        const r = await timedFetch(api, req.url);
        const elapsed = performance.now() - start;
        log(`  #${req.id} ${req.label}: ${r.status} OK ${fmtMs(elapsed)}`);
        perReq.push({ id: req.id, label: req.label, ok: true, ms: elapsed });
      } catch (e) {
        const elapsed = performance.now() - start;
        log(`  #${req.id} ${req.label}: FAIL ${fmtMs(elapsed)} — ${e}`);
        perReq.push({ id: req.id, label: req.label, ok: false, ms: elapsed });
      }
    })
  );

  const totalMs = performance.now() - t0;
  const okCount = perReq.filter((r) => r.ok).length;
  const avgMs =
    perReq.filter((r) => r.ok).reduce((s, r) => s + r.ms, 0) / (okCount || 1);

  log(
    `  Result: ${okCount}/${STRESS_COUNT} OK, total ${fmtMs(
      totalMs
    )}, avg ${fmtMs(avgMs)}/req`
  );

  return {
    name: `Stress httpbin (${STRESS_COUNT})`,
    ok: okCount === STRESS_COUNT,
    ms: totalMs,
    okCount,
    total: STRESS_COUNT,
    avgMs,
    perReq,
  };
}

// Summary

function printSummary(results) {
  log("");
  log("================================================================");
  log(`  smolmix-wasm test results`);
  log(`  Config: ${CONFIG_LABEL}`);
  log(`  Date:   ${new Date().toISOString()}`);
  log("================================================================");
  log("");

  const nameWidth = 28;
  const resultWidth = 10;

  log(`  ${"Test".padEnd(nameWidth)}${"Result".padEnd(resultWidth)}Time`);
  log(
    `  ${"".padEnd(nameWidth, "-")}${"".padEnd(resultWidth, "-")}${"".padEnd(
      20,
      "-"
    )}`
  );

  for (const r of results) {
    let resultStr;
    if (r.total !== undefined) {
      resultStr = `${r.okCount}/${r.total}`;
    } else {
      resultStr = r.ok ? "PASS" : "FAIL";
    }

    let timeStr = r.ms ? fmtMs(r.ms) : "N/A";
    if (r.avgMs !== undefined) {
      timeStr += `  (avg ${fmtMs(r.avgMs)}/req)`;
    }

    log(
      `  ${r.name.padEnd(nameWidth)}${resultStr.padEnd(resultWidth)}${timeStr}`
    );
  }

  log("");
  log("================================================================");

  // Also output as JSON for programmatic consumption
  const json = {
    config: { cover: ENABLE_COVER, poisson: ENABLE_POISSON },
    date: new Date().toISOString(),
    results: results.map((r) => ({
      name: r.name,
      ok: r.ok,
      ms: Math.round(r.ms),
      ...(r.okCount !== undefined && { okCount: r.okCount, total: r.total }),
      ...(r.avgMs !== undefined && { avgMs: Math.round(r.avgMs) }),
      ...(r.error && { error: r.error }),
    })),
  };
  // Machine-readable output for Playwright (bypasses log() timestamp prefix)
  console.log("RESULTS_JSON:" + JSON.stringify(json));
}

// Main

async function runSuite() {
  log("smolmix-wasm headless test runner");
  log(`Config: ${CONFIG_LABEL}`);
  log(`Stress count: ${STRESS_COUNT}`);
  log(`IPR: ${IPR_ADDRESS.slice(0, 40)}...`);
  log("");

  // 1. Start worker
  log("Starting worker...");
  let api;
  try {
    api = await createWorker();
    log("Worker started");
  } catch (e) {
    log(`FATAL: Worker creation failed: ${e}`);
    printSummary([{ name: "Worker init", ok: false, ms: 0, error: String(e) }]);
    return;
  }

  // 2. Setup tunnel
  log("Setting up tunnel...");
  const setupT0 = performance.now();
  try {
    await api.setupMixTunnel({
      preferredIpr: IPR_ADDRESS,
      clientId: "headless-" + Math.random().toString(36).slice(2, 8),
      forceTls: true,
      disablePoissonTraffic: !ENABLE_POISSON,
      disableCoverTraffic: !ENABLE_COVER,
    });
    const setupMs = performance.now() - setupT0;
    log(`Tunnel ready in ${fmtMs(setupMs)}`);
  } catch (e) {
    log(`FATAL: Tunnel setup failed: ${e}`);
    printSummary([
      { name: "Tunnel setup", ok: false, ms: 0, error: String(e) },
    ]);
    return;
  }

  // 3. Run tests sequentially
  const results = [];

  results.push(await testSmoke(api));
  results.push(await testHttpsGetWarm(api));
  results.push(await testStressHttpbin(api));

  // 4. Summary
  printSummary(results);

  // 5. Disconnect
  log("");
  log("Disconnecting...");
  try {
    await api.disconnectMixTunnel();
    log("Disconnected");
  } catch (e) {
    log(`Disconnect error: ${e}`);
  }

  log("Done.");
}

runSuite();
