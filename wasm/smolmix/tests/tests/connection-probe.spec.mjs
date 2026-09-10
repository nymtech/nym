// Connection probe: run tunnel establishment many times against random exits,
// classify each run from the console logs the tunnel already emits, probe one
// DoH resolver per run (rotating), and write results.md.
//
// Not part of smoke/suite: it is slow and real-network. Run explicitly:
//   PROBE_RUNS=100 npx playwright test --project=connection-probe
//
// Needs a debug build (the harness leaves the internal-dev debug checkbox checked
// so the v10->v9 downgrade line, a debug log, is emitted). See
// openspec/changes/add-connection-probe-harness.

import { test, expect } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const RUNS = parseInt(process.env.PROBE_RUNS || "100", 10);
// Rotated one per run so each resolver is sampled ~RUNS/3 times. dohEndpoints is
// fixed at setup and the tunnel is one-shot, so one connection cannot test all
// three; this is the equivalent that keeps the run count at RUNS.
const RESOLVERS = ["1.1.1.1", "9.9.9.9", "8.8.8.8"];
const DNS_HOST = process.env.PROBE_HOST || "example.com";
const SETUP_TIMEOUT_MS = 150_000;
const DNS_TIMEOUT_MS = 40_000;

// Timestamped per run so successive runs accumulate instead of overwriting,
// under a results/ subdir so they don't clutter the tests root.
const RESULTS_DIR = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "results",
);
// Local-time stamp `YYYY-MM-DD_HHMMSS`: sorts in time order, filesystem-safe
// (no colons), and readable, unlike a raw unix timestamp.
const now = new Date();
const pad = (n) => String(n).padStart(2, "0");
const STAMP = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
const RESULTS_PATH = path.join(RESULTS_DIR, `results-${STAMP}.md`);

// Poll a live console buffer so a line that arrived just before we start waiting
// is still seen. Resolves with the first matching line, rejects on timeout.
function waitForLine(buffer, predicate, timeoutMs) {
  return new Promise((resolve, reject) => {
    const start = Date.now();
    const tick = () => {
      const hit = buffer.find(predicate);
      if (hit) return resolve(hit);
      if (Date.now() - start > timeoutMs) return reject(new Error("timeout"));
      setTimeout(tick, 200);
    };
    tick();
  });
}

function classifyConnection(lines) {
  const text = lines.join("\n");
  const gateway = (text.match(/registration with gateway (\S+)/) || [])[1] || "";
  const candidates = (text.match(/auto-discovered (\d+) IPR candidate/) || [])[1] || "";
  const attempts = (text.match(/\[smolmix\] connecting to IPR /g) || []).length;
  const downgrade = /v10 connect timed out; retrying v9/.test(text);
  const rotated = attempts > 1;

  const connected = text.match(/IPR connected: (\S+) \(in ([^)]+)\)/);
  if (connected) {
    // MTU: Some(n) is v10, None is v9. It is a debug log; fall back to the
    // downgrade signal if it is absent.
    const mtu = (text.match(/MTU: Some\((\d+)\)/) || [])[1];
    const version = mtu ? "v10" : downgrade ? "v9" : "v9?";
    return {
      outcome: "connected",
      gateway,
      candidates,
      exit: connected[1],
      version,
      mtu: mtu || "",
      attempts,
      downgrade,
      rotated,
      ms: connected[2],
      reason: "",
    };
  }

  let reason = "unknown";
  if (/no v9-capable IPRs available/.test(text)) reason = "no-eligible-exits";
  else if (/registration|gateway/i.test(text) && /fail|error/i.test(text)) reason = "gateway-registration";
  else if (attempts > 0) reason = "attempts-exhausted";
  return {
    outcome: "failed",
    gateway,
    candidates,
    exit: "",
    version: "",
    mtu: "",
    attempts,
    downgrade,
    rotated,
    ms: "",
    reason,
  };
}

// The verdict is the aggregation bucket; `status` carries the actual DoH HTTP
// code (e.g. 505), so a resolver that resolved is distinguishable from one that
// failed with a specific status, instead of flattening every non-200 to `server-error`.
// `location` is the redirect target when a resolver answers a 3xx, so a resolver
// that redirects (captive portal, moved endpoint) shows in the results.
//
// `text` is the run's DoH console lines (`[dns] resolver ... => HTTP <code>`,
// emitted for every response, plus a `redirect ... Location:` line on a 3xx)
// concatenated with the UI panel, which carries the `=> <ip>` success line and
// any thrown-error string.
function classifyDns(text, resolver) {
  const t = text || "";
  const status = (t.match(/=> HTTP (\d{3})/) || t.match(/HTTP (\d{3})/) || [])[1] || "";
  const location = (t.match(/redirect.*Location: (.+)/) || [])[1] || "";
  const resolved = /resolved '/.test(t) || (/=>\s*\d/.test(t) && !/failed/.test(t));
  if (/429/.test(t) || status === "429") return { resolver, verdict: "rate-limited", status: status || "429", location };
  if (location || /^3\d\d$/.test(status)) return { resolver, verdict: "redirect", status, location };
  if (resolved) return { resolver, verdict: "resolved", status, location };
  if (/timed out|timeout/i.test(t)) return { resolver, verdict: "timeout", status, location };
  if (status) return { resolver, verdict: "server-error", status, location };
  if (/failed/.test(t)) return { resolver, verdict: "error", status, location };
  return { resolver, verdict: "no-result", status, location };
}

// Substrings that mark a console line as an error or anomaly worth keeping. The
// list stays narrow so benign debug noise (e.g. the missing-close_notify line)
// does not match. `pageerror` covers uncaught exceptions, which the console
// listener alone misses.
const ERROR_MARKERS = [
  "setupMixTunnel failed",
  "FATAL",
  "resolve failed",
  "returned HTTP",
  "rate-limited",
  "redirect",
  "malformed",
  "handshake FAILED",
  "pageerror",
  "panicked at",
  "request failed",
  "insufficient to route",
];

// Pull the verbatim error/anomaly lines out of a run's console buffer, so the
// results file preserves the actual failure text for diagnosis afterwards, not
// just a verdict bucket. Deduped; each line capped so one pathological message
// cannot blow up the file.
function collectErrors(lines) {
  const seen = new Set();
  const out = [];
  for (const line of lines) {
    if (!ERROR_MARKERS.some((m) => line.includes(m))) continue;
    const trimmed = line.replace(/\s+/g, " ").trim().slice(0, 500);
    if (seen.has(trimmed)) continue;
    seen.add(trimmed);
    out.push(trimmed);
  }
  return out;
}

// Wall-clock HH:MM:SS at the moment of the call. Recorded per run so a block of
// failures can be lined up against epoch boundaries (a topology outage clusters
// in time; scattered failures do not).
function timeOfDay() {
  const d = new Date();
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

async function runOnce(page, buffer, i) {
  buffer.length = 0;
  const ts = timeOfDay();
  const resolver = RESOLVERS[i % RESOLVERS.length];

  await page.goto("http://localhost:9001");
  await page.waitForSelector("#btn-setup");

  // Auto-discovery, fresh client id (fresh gateway + random exit), one pinned
  // DoH resolver for this run.
  await page.check("#opt-random-ipr").catch(() => {});
  await page.fill("#opt-client-id", `probe-${i}-${Math.random().toString(36).slice(2, 8)}`);
  await page.fill("#opt-doh-endpoints", `https://${resolver}/dns-query`);

  const ready = waitForLine(buffer, (t) => t.includes("tunnel ready") || t.includes("setupMixTunnel OK"), SETUP_TIMEOUT_MS);
  const failed = waitForLine(buffer, (t) => t.includes("setupMixTunnel failed") || t.includes("FATAL"), SETUP_TIMEOUT_MS);
  await page.click("#btn-setup");

  let connectionOk = false;
  try {
    const outcome = await Promise.race([
      ready.then(() => "ok"),
      failed.then(() => "fail"),
    ]);
    connectionOk = outcome === "ok";
  } catch {
    // neither line within the budget: treat as a failed run, classify from logs
  }

  const conn = classifyConnection(buffer);

  let dns = { resolver, verdict: connectionOk ? "not-run" : "skipped-no-tunnel" };
  if (connectionOk) {
    await page.fill("#dns-host", DNS_HOST);
    await page.click("#btn-dns-tunnel");
    try {
      await waitForLine(buffer, (t) => t.includes(`resolved '${DNS_HOST}'`) || t.includes("rate-limited") || t.includes("resolve failed") || t.includes("returned HTTP"), DNS_TIMEOUT_MS);
    } catch {
      // fall through; read whatever the dns-log shows
    }
    const dnsLog = (await page.textContent("#dns-log").catch(() => "")) || "";
    // The DoH HTTP status and any redirect Location come from the wasm `[dns]`
    // console lines, not the UI panel, so classify from both together.
    const dnsConsole = buffer.filter((t) => t.includes("[dns]")).join("\n");
    dns = classifyDns(`${dnsConsole}\n${dnsLog}`, resolver);
  }

  return { run: i + 1, ts, ...conn, dnsResolver: dns.resolver, dnsVerdict: dns.verdict, dnsStatus: dns.status || "", dnsLocation: dns.location || "", errors: collectErrors(buffer) };
}

function pct(n, d) {
  return d === 0 ? "n/a" : `${((100 * n) / d).toFixed(0)}%`;
}

function writeResults(rows) {
  const connected = rows.filter((r) => r.outcome === "connected");
  const attempts = connected.map((r) => r.attempts).filter((n) => n > 0);
  const meanAttempts = attempts.length ? (attempts.reduce((a, b) => a + b, 0) / attempts.length).toFixed(2) : "n/a";
  const maxAttempts = attempts.length ? Math.max(...attempts) : "n/a";

  // A run that failed because a whole mixnet routing layer was empty is a network
  // condition, not a tunnel fault, so report a second rate that excludes it. These
  // cluster in time (see the timestamps), which is the epoch-rollover signature.
  const topologyDown = rows.filter((r) => (r.errors || []).some((e) => e.includes("insufficient to route")));
  const routable = rows.length - topologyDown.length;

  const perResolver = {};
  for (const r of RESOLVERS) perResolver[r] = { resolved: 0, "rate-limited": 0, redirect: 0, timeout: 0, "server-error": 0, error: 0, "no-result": 0 };
  for (const r of rows) {
    if (r.dnsVerdict && perResolver[r.dnsResolver] && perResolver[r.dnsResolver][r.dnsVerdict] !== undefined) {
      perResolver[r.dnsResolver][r.dnsVerdict] += 1;
    }
  }

  const lines = [];
  lines.push("# Connection probe results", "");
  lines.push(`Runs: ${rows.length}. Host: \`${DNS_HOST}\`.`, "");

  lines.push("## Summary", "");
  lines.push("| metric | value |", "| --- | --- |");
  lines.push(`| connection success rate | ${pct(connected.length, rows.length)} (${connected.length}/${rows.length}) |`);
  lines.push(`| connection success excl. topology-down | ${pct(connected.length, routable)} (${connected.length}/${routable}) |`);
  lines.push(`| topology-down runs (insufficient to route) | ${topologyDown.length} |`);
  lines.push(`| runs with a v10->v9 downgrade | ${pct(rows.filter((r) => r.downgrade).length, rows.length)} |`);
  lines.push(`| runs that rotated exits | ${pct(rows.filter((r) => r.rotated).length, rows.length)} |`);
  lines.push(`| mean attempts (successful) | ${meanAttempts} |`);
  lines.push(`| max attempts | ${maxAttempts} |`, "");

  lines.push("## Per-resolver DoH verdicts", "");
  lines.push("| resolver | resolved | rate-limited (429) | redirect (3xx) | timeout | server-error | other |", "| --- | --- | --- | --- | --- | --- | --- |");
  for (const r of RESOLVERS) {
    const p = perResolver[r];
    lines.push(`| ${r} | ${p.resolved} | ${p["rate-limited"]} | ${p.redirect} | ${p.timeout} | ${p["server-error"]} | ${p.error + p["no-result"]} |`);
  }
  lines.push("");

  lines.push("## Connection runs", "");
  lines.push("| run | time | entry gw | exit | ver | mtu | cand | attempts | downgrade | rotated | outcome | ms | reason | dns resolver | dns verdict | dns status | err |");
  lines.push("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
  for (const r of rows) {
    const short = (s) => (s ? `${s.slice(0, 8)}...` : "");
    // Fold a redirect target into the status cell (e.g. `301 → host`) so the
    // table width stays the same and redirects stay readable.
    const dnsStatus = r.dnsLocation ? `${r.dnsStatus} → ${r.dnsLocation}` : r.dnsStatus || "";
    lines.push(
      `| ${r.run} | ${r.ts || ""} | ${short(r.gateway)} | ${short(r.exit)} | ${r.version} | ${r.mtu} | ${r.candidates} | ${r.attempts} | ${r.downgrade ? "yes" : ""} | ${r.rotated ? "yes" : ""} | ${r.outcome} | ${r.ms} | ${r.reason} | ${r.dnsResolver} | ${r.dnsVerdict} | ${dnsStatus} | ${r.errors?.length || ""} |`,
    );
  }
  lines.push("");

  // Verbatim error/anomaly lines per run, so you can diagnose a failure from the
  // results file alone without a re-run. Present only when a run went wrong.
  const errorRows = rows.filter((r) => r.errors && r.errors.length);
  if (errorRows.length) {
    lines.push("## Errors", "");
    for (const r of errorRows) {
      for (const e of r.errors) {
        lines.push(`- run ${r.run} (${r.dnsResolver}, ${r.outcome}): ${e}`);
      }
    }
    lines.push("");
  }

  fs.mkdirSync(RESULTS_DIR, { recursive: true });
  fs.writeFileSync(RESULTS_PATH, lines.join("\n"));
}

test("connection probe", async ({ page }) => {
  test.setTimeout(RUNS * (SETUP_TIMEOUT_MS + DNS_TIMEOUT_MS) + 60_000);

  const buffer = [];
  page.on("console", (msg) => {
    const t = msg.text();
    buffer.push(t);
    if (t.startsWith("[")) console.log(t);
  });
  // Uncaught page/worker exceptions never reach the console listener, so capture
  // them into the same per-run buffer where collectErrors can find them.
  page.on("pageerror", (err) => buffer.push(`[pageerror] ${err.message || err}`));

  const rows = [];
  for (let i = 0; i < RUNS; i++) {
    try {
      const row = await runOnce(page, buffer, i);
      rows.push(row);
      console.log(`[probe] run ${i + 1}/${RUNS}: ${row.outcome} attempts=${row.attempts} downgrade=${row.downgrade} dns(${row.dnsResolver})=${row.dnsVerdict}`);
    } catch (e) {
      rows.push({ run: i + 1, ts: timeOfDay(), outcome: "error", gateway: "", exit: "", version: "", mtu: "", candidates: "", attempts: 0, downgrade: false, rotated: false, ms: "", reason: String(e).slice(0, 40), dnsResolver: RESOLVERS[i % RESOLVERS.length], dnsVerdict: "n/a", dnsStatus: "", dnsLocation: "", errors: [String(e).replace(/\s+/g, " ").trim().slice(0, 500), ...collectErrors(buffer)] });
      console.log(`[probe] run ${i + 1}/${RUNS}: harness error ${e}`);
    }
    writeResults(rows); // write incrementally so a mid-run abort still leaves data
  }

  writeResults(rows);
  console.log(`[probe] wrote ${RESULTS_PATH}`);
  expect(rows.length).toBe(RUNS);
});
