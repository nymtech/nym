// Display metadata for node kinds and transport modes, shared by the legend
// and the diagram engine so symbols stay consistent.

import type { Mode, NodeKind } from "./types";

export interface NodeMeta {
  label: string;
  short: string;
  /** CSS variable used to fill the node. */
  colorVar: string;
  /** SVG glyph shape. */
  shape: "circle" | "square" | "diamond" | "hex";
}

export const NODE_META: Record<NodeKind, NodeMeta> = {
  client: { label: "Client", short: "Client", colorVar: "--node-client", shape: "circle" },
  vpn: { label: "VPN server", short: "VPN", colorVar: "--node-vpn", shape: "square" },
  entry: { label: "Nym entry gateway", short: "Entry GW", colorVar: "--node-gateway", shape: "square" },
  mix: { label: "Nym mix node", short: "Mix", colorVar: "--node-mix", shape: "hex" },
  exit: { label: "Nym exit gateway", short: "Exit GW", colorVar: "--node-gateway", shape: "square" },
  ipr: { label: "IP Packet Router", short: "IPR", colorVar: "--node-ipr", shape: "diamond" },
  "service-provider": { label: "Nym service provider (SURB replies)", short: "Service provider", colorVar: "--node-sp", shape: "diamond" },
  destination: { label: "Destination (observer)", short: "Destination", colorVar: "--node-destination", shape: "square" },
  internet: { label: "Open internet", short: "Internet", colorVar: "--node-internet", shape: "circle" },
};

export interface ModeMeta {
  label: string;
  colorVar: string;
  /** Dashed line = timing NOT obfuscated; solid = mixnet mixing. */
  dashed: boolean;
  description: string;
}

export const MODE_META: Record<Mode, ModeMeta> = {
  direct: {
    label: "Direct",
    colorVar: "--mode-direct",
    dashed: true,
    description: "Unprotected connection — no IP or timing protection.",
  },
  vpn: {
    label: "VPN",
    colorVar: "--mode-vpn",
    dashed: true,
    description: "Hides client IP; no mixing delays or timing protection.",
  },
  dvpn: {
    label: "dVPN (WireGuard)",
    colorVar: "--mode-dvpn",
    dashed: true,
    description: "Fast, line-rate; hides IP; no timing obfuscation.",
  },
  mixnet: {
    label: "Mixnet (5-hop)",
    colorVar: "--mode-mixnet",
    dashed: false,
    description: "Mixing delays + cover traffic + location privacy; slow.",
  },
};

export const NODE_LEGEND_ORDER: NodeKind[] = [
  "client",
  "vpn",
  "entry",
  "mix",
  "exit",
  "ipr",
  "service-provider",
  "destination",
  "internet",
];
