import fs from 'node:fs';
import path from 'node:path';
import type { Page, TestInfo } from '@playwright/test';

/**
 * Visual flow report for the Node Families e2e journeys. Each `shot()` writes a numbered,
 * captioned PNG into `e2e-report/screenshots/<test>/` (and attaches it to the Playwright HTML
 * report). `buildGallery()` (run from globalTeardown) stitches them into a static
 * `e2e-report/index.html` filmstrip for visual inspection / smoke checks — uploaded by CI.
 */

export const REPORT_DIR = path.resolve(__dirname, '..', '..', 'e2e-report');
const SCREENSHOTS_DIR = path.join(REPORT_DIR, 'screenshots');

const slug = (s: string) =>
  s
    .toLowerCase()
    .replace(/[^\w]+/g, '-')
    .replace(/^-+|-+$/g, '');

// Per-test step counters (workers run one test each, so this stays consistent per test).
const counters = new Map<string, number>();

/** Clear any previous report so the upload reflects only the latest run. */
export function resetReport(): void {
  fs.rmSync(REPORT_DIR, { recursive: true, force: true });
  fs.mkdirSync(SCREENSHOTS_DIR, { recursive: true });
  counters.clear();
}

/** Capture a full-page, captioned step screenshot into the report (and the Playwright report). */
export async function shot(page: Page, testInfo: TestInfo, label: string): Promise<void> {
  const testSlug = slug(testInfo.title);
  const dir = path.join(SCREENSHOTS_DIR, testSlug);
  fs.mkdirSync(dir, { recursive: true });
  const n = (counters.get(testSlug) ?? 0) + 1;
  counters.set(testSlug, n);
  const file = `${String(n).padStart(2, '0')}-${slug(label)}.png`;
  const body = await page.screenshot({ fullPage: true });
  fs.writeFileSync(path.join(dir, file), body);
  await testInfo.attach(label, { body, contentType: 'image/png' });
}

/** Stitch the captured screenshots into a static index.html filmstrip. */
export function buildGallery(): void {
  if (!fs.existsSync(SCREENSHOTS_DIR)) return;
  const tests = fs
    .readdirSync(SCREENSHOTS_DIR)
    .filter((d) => fs.statSync(path.join(SCREENSHOTS_DIR, d)).isDirectory())
    .sort();

  const sections = tests
    .map((t) => {
      const imgs = fs
        .readdirSync(path.join(SCREENSHOTS_DIR, t))
        .filter((f) => f.endsWith('.png'))
        .sort();
      const frames = imgs
        .map((f) => {
          const caption = f.replace(/^\d+-/, '').replace(/\.png$/, '').replace(/-/g, ' ');
          return `<figure><img loading="lazy" src="screenshots/${t}/${f}" alt="${caption}"><figcaption>${caption}</figcaption></figure>`;
        })
        .join('\n');
      return `<section><h2>${t.replace(/-/g, ' ')}</h2><div class="strip">${frames}</div></section>`;
    })
    .join('\n');

  const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Node Families e2e — visual flow report</title>
<style>
  :root { color-scheme: dark; }
  body { margin: 0; padding: 24px; font: 14px/1.5 system-ui, sans-serif; background: #0a0a0a; color: #eee; }
  h1 { font-size: 20px; }
  section { margin: 32px 0; }
  h2 { font-size: 16px; color: #5bf0a0; text-transform: capitalize; border-bottom: 1px solid #333; padding-bottom: 6px; }
  .strip { display: flex; gap: 16px; overflow-x: auto; padding: 12px 0; }
  figure { margin: 0; flex: 0 0 auto; width: 320px; }
  img { width: 320px; border: 1px solid #333; border-radius: 6px; background: #1a1a1c; cursor: zoom-in; }
  img:target, img:active { transform: scale(2.4); transform-origin: top left; position: relative; z-index: 10; }
  figcaption { font-size: 12px; color: #aaa; margin-top: 6px; text-transform: capitalize; }
</style>
</head>
<body>
<h1>Node Families e2e — visual flow report</h1>
<p style="color:#888">Per-step screenshots from the mock-wired Playwright journeys (design D1/D2). Hover/scroll each filmstrip; click an image to zoom.</p>
${sections}
</body>
</html>`;
  fs.writeFileSync(path.join(REPORT_DIR, 'index.html'), html);
}
