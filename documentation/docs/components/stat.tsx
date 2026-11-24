// `<Stat name="nodes" />`: a count from the network scrape
// (components/outputs/api-scraping-outputs/nodes-count.json, written by the
// prebuild). A snippet imports nothing, so it names the value. The text outputs
// substitute the same value (lib/retrieval/mdx.mjs, `substitute`).

import STATS from "./outputs/api-scraping-outputs/nodes-count.json";

export function Stat({ name }: { name: string }) {
  const value = (STATS as Record<string, unknown>)[name];
  if (typeof value !== "number" && typeof value !== "string") {
    throw new Error(`<Stat name="${name}">: nodes-count.json has no such value`);
  }
  return <>{String(value)}</>;
}
