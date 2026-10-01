import { describe, expect, it } from "vitest";
import { fileURLToPath, pathToFileURL } from "node:url";
import { dirname, join } from "node:path";
import { configNavMeta, EXAMPLE_NAV } from "./nav";

// Enforce that the Nextra sub-navs are projections of the typed scenario data.
// If a scenario is added to GENERIC_SCENARIOS (or a worked example to the
// registry) without updating the matching _meta.js, these assertions fail.

const HERE = dirname(fileURLToPath(import.meta.url));
const THREAT_MODEL = join(HERE, "../../content/network/threat-model");

// Nextra 4 `_meta` files are ESM (`export default { ... }`), so they are
// imported rather than JSON-parsed. pathToFileURL keeps the absolute path
// loadable by the dynamic import on every platform.
async function readMeta(sub: string): Promise<Record<string, string>> {
  const mod = await import(pathToFileURL(join(THREAT_MODEL, sub, "_meta.js")).href);
  return mod.default;
}

describe("data-derived nav", () => {
  it("configurations/_meta.js is a projection of GENERIC_SCENARIOS", async () => {
    const derived = configNavMeta();
    const meta = await readMeta("configurations");
    // Order matters for the sidebar, so compare the key sequence too.
    expect(Object.keys(meta)).toEqual(Object.keys(derived));
    expect(meta).toEqual(derived);
  });

  it("examples/_meta.js matches the worked-example registry", async () => {
    const meta = await readMeta("examples");
    expect(Object.keys(meta)).toEqual(Object.keys(EXAMPLE_NAV));
    expect(meta).toEqual(EXAMPLE_NAV);
  });
});
