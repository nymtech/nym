#!/usr/bin/env node
// Browser check for the docs site. Opens each page in dark and light mode, then writes a
// screenshot per page and one report.json (console errors, failed and unfinished requests,
// a few computed styles) to documentation/docs/.cache/visual-check/. It then runs a few search
// queries and writes search.json.
//
// Usage (dev server must be running):
//   node documentation/scripts/docs-visual-check.mjs [page ...]
//   BASE=https://<preview-host>/docs node documentation/scripts/docs-visual-check.mjs apis/nym-api
//
// Playwright comes from wasm/smolmix/tests, so the docs package gets no new dependency.
// Browser: $CHROMIUM if set, else /usr/bin/chromium if present, else Playwright's own
// download (`npx playwright install chromium`, run from wasm/smolmix/tests).
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { mkdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const require = createRequire(new URL("../../wasm/smolmix/tests/", import.meta.url));
const { chromium } = require("@playwright/test");

const BASE = (process.env.BASE || "http://localhost:3000/docs").replace(/\/$/, "");
const OUT = fileURLToPath(new URL("../docs/.cache/visual-check/", import.meta.url));
const pages = process.argv.slice(2).length
  ? process.argv.slice(2)
  : ["", "operators/variables", "apis/explorer-api", "apis/nym-api", "developers/rust"];

// Runs in the page. Reads the values the theme port is most likely to get wrong.
function probe() {
  const css = (el, prop) => (el ? getComputedStyle(el)[prop] : null);
  const sidebar = document.querySelector(".nextra-sidebar");
  return {
    title: document.title,
    htmlClass: document.documentElement.className,
    background: css(document.documentElement, "backgroundColor"),
    linkColour: css(document.querySelector("main a.x\\:text-primary-600"), "color"),
    sidebarWidth: css(sidebar, "width"),
    activeItem: (() => {
      const a = sidebar?.querySelector("a.x\\:bg-primary-100");
      return a && { text: a.textContent, colour: css(a, "color"), borderLeft: css(a, "borderLeft") };
    })(),
    tocVisible: !!document.querySelector(".nextra-toc a"),
    // A heading with no text gives an anchor like "#-1" and an "undefined" title in search.
    emptyHeadings: [...document.querySelectorAll("main :is(h1, h2, h3, h4, h5, h6)")]
      .filter((h) => !h.textContent.trim())
      .map((h) => `${h.outerHTML.slice(0, 160)} | after: ${h.previousElementSibling?.textContent.trim().slice(0, 80)}`),
    copyPageButtons: [...document.querySelectorAll("button")].filter((b) => b.textContent.trim() === "Copy page").length,
    redocLoading: /Loading \.\.\./.test(document.querySelector("main")?.innerText ?? ""),
  };
}

await mkdir(OUT, { recursive: true });
const systemChromium = existsSync("/usr/bin/chromium") ? "/usr/bin/chromium" : undefined;
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM || systemChromium });
const report = [];

// The phone pass opens the menu: Nextra 4 renders it as a separate element from the sidebar.
const passes = [
  { name: "dark", theme: "dark", width: 1400 },
  { name: "light", theme: "light", width: 1400 },
  { name: "mobile", theme: "dark", width: 390 },
];
for (const { name, theme, width } of passes) {
  const context = await browser.newContext({ viewport: { width, height: 1000 } });
  // next-themes reads this key before first paint.
  await context.addInitScript((t) => localStorage.setItem("theme", t), theme);

  for (const path of pages) {
    const page = await context.newPage();
    const entry = { path: path || "/", theme: name, console: [], pageErrors: [], failed: [], badStatus: [] };
    const open = new Set();
    const status = new Map();

    page.on("console", (m) => {
      if (m.type() === "error" || m.type() === "warning") entry.console.push(`${m.type()}: ${m.text()}`);
    });
    page.on("pageerror", (e) => entry.pageErrors.push(String(e)));
    page.on("request", (r) => open.add(r));
    page.on("requestfinished", (r) => open.delete(r));
    page.on("requestfailed", (r) => {
      open.delete(r);
      entry.failed.push(`${r.url()} (${r.failure()?.errorText})`);
    });
    page.on("response", (r) => {
      status.set(r.request(), r.status());
      if (r.status() >= 400) entry.badStatus.push(`${r.status()} ${r.url()}`);
    });

    try {
      await page.goto(`${BASE}/${path}`, { waitUntil: "domcontentloaded", timeout: 60_000 });
      // A page that never goes idle (a hung fetch) is a finding, not a script failure.
      await page.waitForLoadState("networkidle", { timeout: 30_000 }).catch(() => {
        entry.neverIdle = true;
      });
      entry.probe = await page.evaluate(probe);
      if (name === "mobile") await page.locator(".nextra-hamburger").click({ timeout: 5_000 });
    } catch (e) {
      entry.navigationError = String(e);
    }
    // "no response" = the server never answered; a status = headers came, the body did not end.
    entry.unfinished = [...open].map((r) => `${r.method()} ${r.url()} (${status.get(r) ?? "no response"})`);
    entry.screenshot = `${(path || "index").replaceAll("/", "_")}.${name}.png`;
    await page.screenshot({ path: OUT + entry.screenshot }).catch((e) => (entry.screenshotError = String(e)));
    report.push(entry);
    await page.close();
    console.log(`${name.padEnd(6)} /${path}  errors=${entry.console.length + entry.pageErrors.length} failed=${entry.failed.length} unfinished=${entry.unfinished.length}`);
  }
  await context.close();
}

// Search: type each query into the navbar search box and record what the result list shows.
// SEARCH="a,b" replaces the default queries. Selectors are Nextra 4's (Pagefind).
const queries = (process.env.SEARCH || "mixnet,nym-node,smolmix,delegation,wireguard exit policy").split(",");
const search = [];
const context = await browser.newContext({ viewport: { width: 1400, height: 1000 } });
for (const query of queries) {
  // A fresh page per query, so a slow answer to the last query cannot pass for this one.
  const page = await context.newPage();
  const entry = { query };
  try {
    await page.goto(`${BASE}/developers`, { waitUntil: "domcontentloaded", timeout: 60_000 });
    const started = Date.now();
    await page.locator(".nextra-search input").first().fill(query);
    await page
      .waitForFunction(() => {
        const text = document.querySelector(".nextra-search-results")?.innerText.trim();
        return text && !text.startsWith("Loading");
      }, null, { timeout: 15_000 })
      .catch(() => (entry.timedOut = true));
    entry.ms = Date.now() - started;
    Object.assign(
      entry,
      await page.evaluate(() => {
        const box = document.querySelector(".nextra-search-results");
        const hits = [...(box?.querySelectorAll("a") ?? [])].map((a) => ({
          href: a.getAttribute("href"),
          text: a.innerText.replace(/\s+/g, " ").slice(0, 160),
        }));
        return { hits, message: hits.length ? null : (box?.innerText.trim() ?? "no result list in the DOM") };
      }),
    );
  } catch (e) {
    entry.error = String(e);
  }
  entry.screenshot = `search_${query.replaceAll(/\W+/g, "-")}.png`;
  await page.screenshot({ path: OUT + entry.screenshot }).catch(() => {});
  search.push(entry);
  await page.close();
  console.log(`search "${query}"  hits=${entry.hits?.length ?? 0} ${entry.message ?? ""} ${entry.ms ?? "?"}ms`);
}

await browser.close();
await writeFile(OUT + "search.json", JSON.stringify(search, null, 2));
await writeFile(OUT + "report.json", JSON.stringify(report, null, 2));
console.log(`wrote ${OUT}report.json`);
