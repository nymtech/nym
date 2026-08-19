// Reusable shared-node topology builders for the branching configurations
// (multi-exit, hybrid). Ported from the source app and generalised: the end
// node is a generic "destination" and captions name no application.
//
// Columns: 0 client, 1 entry, 2-4 mix layers, 5 exit(s), 6 destination.

import type {
  DiagramNode,
  DiagramRoute,
  NodeKind,
  Topology,
  TrafficDirection,
} from "./types";

const MIX_NODES: DiagramNode[] = [
  { id: "mix1", kind: "mix", col: 2, label: "Mix L1" },
  { id: "mix2", kind: "mix", col: 3, label: "Mix L2" },
  { id: "mix3", kind: "mix", col: 4, label: "Mix L3" },
];
const MIX_IDS = ["mix1", "mix2", "mix3"];

/** One client, a single mixnet route to an end node, with a SURB reply back. */
export function mixnetSurbTopology(opts: {
  endId: string;
  endKind: NodeKind;
  endLabel?: string;
  caption?: string;
}): Topology {
  const nodes: DiagramNode[] = [
    { id: "client", kind: "client", col: 0 },
    { id: "entry", kind: "entry", col: 1 },
    ...MIX_NODES,
    { id: "exit", kind: "exit", col: 5 },
    { id: opts.endId, kind: opts.endKind, col: 6, label: opts.endLabel },
  ];
  const routes: DiagramRoute[] = [
    {
      id: "forward",
      label: "Request",
      mode: "mixnet",
      nodeIds: ["client", "entry", ...MIX_IDS, "exit", opts.endId],
    },
    {
      id: "surb",
      label: "SURB reply",
      mode: "mixnet",
      surb: true,
      nodeIds: [opts.endId, "exit", "mix3", "mix2", "mix1", "entry", "client"],
    },
  ];
  return { nodes, routes, caption: opts.caption };
}

/** One client + entry, a mixnet, multiple exit gateways, one destination. */
export function multipathMixnetTopology(
  exits = 3,
  endId = "dest",
  endKind: NodeKind = "destination",
): Topology {
  const exitNodes: DiagramNode[] = Array.from({ length: exits }, (_, i) => ({
    id: `exit${i}`,
    kind: "exit",
    col: 5,
    label: `Exit ${String.fromCharCode(65 + i)}`,
  }));
  const nodes: DiagramNode[] = [
    { id: "client", kind: "client", col: 0 },
    { id: "entry", kind: "entry", col: 1 },
    ...MIX_NODES,
    ...exitNodes,
    { id: endId, kind: endKind, col: 6 },
  ];
  const routes: DiagramRoute[] = exitNodes.map((ex, i) => ({
    id: `r${i}`,
    label: `Request via ${ex.label}`,
    mode: "mixnet",
    direction: "mixed",
    downloadRatio: 0.5,
    nodeIds: ["client", "entry", ...MIX_IDS, ex.id, endId],
  }));
  return {
    nodes,
    routes,
    caption:
      "One client and entry gateway; requests are spread across several exit gateways over the mixnet, all reaching one destination. Responses return through varied mixnet paths.",
  };
}

/** One client + entry, multiple dVPN exit gateways, one destination. */
export function multipathDvpnTopology(
  exits = 3,
  endId = "dest",
  endKind: NodeKind = "destination",
): Topology {
  const exitNodes: DiagramNode[] = Array.from({ length: exits }, (_, i) => ({
    id: `exit${i}`,
    kind: "exit",
    col: 2,
    label: `Exit ${String.fromCharCode(65 + i)}`,
  }));
  const nodes: DiagramNode[] = [
    { id: "client", kind: "client", col: 0 },
    { id: "entry", kind: "entry", col: 1 },
    ...exitNodes,
    { id: endId, kind: endKind, col: 3 },
  ];
  const routes: DiagramRoute[] = exitNodes.map((ex, i) => ({
    id: `r${i}`,
    label: `Request via ${ex.label}`,
    mode: "dvpn",
    direction: "mixed",
    downloadRatio: 0.8,
    nodeIds: ["client", "entry", ex.id, endId],
  }));
  return {
    nodes,
    routes,
    caption:
      "One client connected to one entry gateway; requests take dVPN routes to different exit gateways, all reaching a single destination.",
  };
}

/**
 * One client + entry, a dVPN exit and one or more mixnet exits, one destination.
 * Used by the recommended hybrid configuration.
 */
export function hybridTopology(opts: {
  endId?: string;
  endKind?: NodeKind;
  endLabel?: string;
  mixExits?: number;
  mixReplies?: boolean;
  caption?: string;
} = {}): Topology {
  const endId = opts.endId ?? "dest";
  const mixExits = opts.mixExits ?? 1;
  const single = mixExits === 1;
  const mixExitNodes: DiagramNode[] = Array.from({ length: mixExits }, (_, i) => ({
    id: single ? "exitMix" : `exitMix${i}`,
    kind: "exit",
    col: 5,
    label: single ? "Exit GW (mixnet)" : `Mixnet exit ${String.fromCharCode(65 + i)}`,
  }));
  const nodes: DiagramNode[] = [
    { id: "client", kind: "client", col: 0 },
    { id: "entry", kind: "entry", col: 1 },
    ...MIX_NODES,
    ...mixExitNodes,
    { id: "exitDvpn", kind: "exit", col: 5, label: "Exit GW (dVPN)" },
    { id: endId, kind: opts.endKind ?? "destination", col: 6, label: opts.endLabel },
  ];
  const routes: DiagramRoute[] = [
    {
      id: "bulk",
      label: "Bulk (dVPN)",
      mode: "dvpn",
      orthogonal: true,
      direction: "mixed",
      downloadRatio: 0.9,
      nodeIds: ["client", "entry", "exitDvpn", endId],
    },
    ...mixExitNodes.map((ex, i) => ({
      id: single ? "sensitive" : `sensitive${i}`,
      label: single ? "Sensitive (mixnet)" : `Sensitive via ${ex.label}`,
      mode: "mixnet" as const,
      direction: (opts.mixReplies ? "mixed" : "upload") as TrafficDirection,
      ...(opts.mixReplies ? { downloadRatio: 0.35 } : {}),
      nodeIds: ["client", "entry", ...MIX_IDS, ex.id, endId],
    })),
  ];
  return { nodes, routes, caption: opts.caption };
}
