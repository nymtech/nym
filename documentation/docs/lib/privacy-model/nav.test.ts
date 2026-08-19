import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { configNavMeta, EXAMPLE_NAV } from "./nav";

// Enforce that the Nextra sub-navs are projections of the typed scenario data.
// If a scenario is added to GENERIC_SCENARIOS (or a worked example to the
// registry) without updating the matching _meta.json, these assertions fail.

const HERE = dirname(fileURLToPath(import.meta.url));
const THREAT_MODEL = join(HERE, "../../pages/network/threat-model");

function readMeta(sub: string): Record<string, string> {
  return JSON.parse(readFileSync(join(THREAT_MODEL, sub, "_meta.json"), "utf8"));
}

describe("data-derived nav", () => {
  it("configurations/_meta.json is a projection of GENERIC_SCENARIOS", () => {
    const derived = configNavMeta();
    const meta = readMeta("configurations");
    // Order matters for the sidebar, so compare the key sequence too.
    expect(Object.keys(meta)).toEqual(Object.keys(derived));
    expect(meta).toEqual(derived);
  });

  it("examples/_meta.json matches the worked-example registry", () => {
    const meta = readMeta("examples");
    expect(Object.keys(meta)).toEqual(Object.keys(EXAMPLE_NAV));
    expect(meta).toEqual(EXAMPLE_NAV);
  });
});
