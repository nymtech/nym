// Turns a diagram item into a shared-node topology for rendering. Branching
// items (multipath, hybrid) carry an explicit `topology`; linear items derive a
// simple chain from their `paths`.
//
// Per-example node labels (item.nodeLabels) are applied here so a generic
// "destination" node can render as "lightwalletd", "chat server", etc. without
// the engine knowing any application name.

import type {
  Architecture,
  DiagramNode,
  DiagramRoute,
  NodeKind,
  Scenario,
  Topology,
} from "./types";

export function buildTopology(item: Scenario | Architecture): Topology {
  const labels = item.nodeLabels;
  if (item.topology) {
    if (!labels) return item.topology;
    // Apply per-example labels to any node that has not set one explicitly.
    return {
      ...item.topology,
      nodes: item.topology.nodes.map((n) => ({
        ...n,
        label: n.label ?? labels[n.kind],
      })),
    };
  }

  const nodes: DiagramNode[] = [];
  const routes: DiagramRoute[] = [];
  item.paths.forEach((p, pi) => {
    const nodeIds = p.stages.map((kind, i) => {
      const id = `p${pi}n${i}`;
      nodes.push({ id, kind, col: i, label: labels?.[kind] });
      return id;
    });
    routes.push({
      id: p.id,
      label: p.label,
      mode: p.mode,
      nodeIds,
      note: p.note,
    });
  });
  return { nodes, routes };
}

/** Map a route's node ids back to node kinds (for latency etc.). */
export function routeStages(topo: Topology, route: DiagramRoute): NodeKind[] {
  const byId = new Map(topo.nodes.map((n) => [n.id, n.kind]));
  return route.nodeIds.map((id) => byId.get(id) as NodeKind);
}

/** The distinct node kinds actually drawn for an item (for the legend). */
export function diagramNodeKinds(item: Scenario | Architecture): NodeKind[] {
  const topo = buildTopology(item);
  return Array.from(new Set(topo.nodes.map((n) => n.kind)));
}
