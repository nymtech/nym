// Scenario variants: generic scenarios that a page draws another way (the type is
// in ../types). Each one keeps the routes and claims of its base scenario, so a
// variant states no new verdict. The pages cite them as
// `<Snippet id="nym/threat-model/variant/<id>" view="diagram" />`.

import type { Scenario, ScenarioVariant } from "../types";
import { requireGenericScenario } from "./generic";

export const SCENARIO_VARIANTS: ScenarioVariant[] = [
  {
    // developers/index: proxy mode, with each mix layer and the exit gateway drawn.
    id: "developers-proxy",
    base: "mixnet",
    topology: {
      nodes: [
        { id: "c", kind: "client", col: 0 },
        { id: "e", kind: "entry", col: 1 },
        { id: "m1", kind: "mix", col: 2 },
        { id: "m2", kind: "mix", col: 3 },
        { id: "m3", kind: "mix", col: 4 },
        { id: "x", kind: "exit", col: 5, label: "Exit GW" },
        { id: "d", kind: "destination", col: 6 },
      ],
      routes: [{ id: "mix", mode: "mixnet", nodeIds: ["c", "e", "m1", "m2", "m3", "x", "d"] }],
    },
  },
  {
    // developers/index: the client runs in the browser.
    id: "developers-browser",
    base: "mixnet",
    nodeLabels: { client: "Browser + WASM client" },
  },
  {
    // developers/service-providers: the end-to-end route ends at the provider's
    // own embedded client, which the diagram draws as a service provider.
    id: "service-provider",
    base: "end-to-end",
    nodes: { p: { kind: "service-provider", label: "Your service provider" } },
    caption:
      "The request crosses the mixnet to the provider's own gateway. There is no exit gateway and no clearnet hop. The reply returns over a SURB, so the provider never learns the client address.",
  },
];

/** The full scenario a variant draws: its base, with the variant's diagram fields. */
export function resolveScenarioVariant(v: ScenarioVariant): Scenario {
  const base = requireGenericScenario(v.base);
  const out: Scenario = { ...base };
  if (v.topology) {
    if (v.nodes || v.caption !== undefined) throw new Error(`scenario variant ${v.id}: a topology of its own takes no nodes or caption; put them in the topology`);
    out.topology = v.topology;
  } else if (v.nodes || v.caption !== undefined) {
    if (!base.topology) throw new Error(`scenario variant ${v.id}: base ${v.base} has no topology to change`);
    const known = new Set(base.topology.nodes.map((n) => n.id));
    for (const id of Object.keys(v.nodes ?? {})) {
      if (!known.has(id)) throw new Error(`scenario variant ${v.id}: base ${v.base} has no node ${id}`);
    }
    out.topology = {
      ...base.topology,
      nodes: base.topology.nodes.map((n) => (v.nodes?.[n.id] ? { ...n, ...v.nodes[n.id] } : n)),
      ...(v.caption !== undefined ? { caption: v.caption } : {}),
    };
  }
  if (v.nodeLabels) out.nodeLabels = v.nodeLabels;
  return out;
}
