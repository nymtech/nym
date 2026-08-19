// Illustrative latency model. NOT measured values, a pedagogical model:
// total = Σ geographic propagation per hop + Σ per-mix-layer mixing delay
// (mixnet only). dVPN/direct incur propagation only; mixnet adds mixing delay
// at each mix layer (the "5-hop + mixing delays" cost).

import type { Mode, NodeKind } from "./types";

/** Geographic "speed-of-light" propagation bands, ~40–300 ms. */
export const GEO_BANDS = [
  { id: "regional", label: "Regional", ms: 40 },
  { id: "continental", label: "Continental", ms: 120 },
  { id: "intercontinental", label: "Intercontinental", ms: 300 },
] as const;

export type GeoBandId = (typeof GEO_BANDS)[number]["id"];

export const DEFAULT_GEO_MS = 120; // continental default
export const DEFAULT_MIX_DELAY_MS = 50; // mean per-layer mixing delay
// Bounded mixnet send rate: packets are metered onto the mixnet, so a message
// fragmented into N Sphinx packets takes (N-1) intervals longer to complete.
export const SEND_INTERVAL_MS = 50;

/** Extra latency to deliver a message fragmented into `packets` Sphinx packets. */
export function fragmentationMs(
  packets: number,
  intervalMs: number = SEND_INTERVAL_MS,
): number {
  return Math.max(0, packets - 1) * intervalMs;
}

export interface LatencyParams {
  /** Geographic propagation per hop (ms), within GEO_BANDS range. */
  geoMsPerHop: number;
  /** Mean mixing delay applied at each mix layer (ms), mixnet only. */
  mixDelayMs: number;
}

export const DEFAULT_LATENCY_PARAMS: LatencyParams = {
  geoMsPerHop: DEFAULT_GEO_MS,
  mixDelayMs: DEFAULT_MIX_DELAY_MS,
};

export interface PathLatency {
  hops: number;
  mixLayers: number;
  /** Forward geographic propagation. */
  propagationMs: number;
  /** Forward mixing delay (mixnet only). */
  mixingMs: number;
  /**
   * Reliable-channel ACK: the acknowledgement follows the original path back,
   * incurring geographic propagation only (no mixing delay). Mixnet only.
   */
  ackMs: number;
  totalMs: number;
}

/**
 * Compute latency for an ordered set of node stages under a transport mode.
 * Mixing delay is added once per `mix` stage, and only when mode === "mixnet".
 * Mixnet paths also incur a reliable-channel ACK on the return trip: geographic
 * propagation over the same hops, with no mixing delay.
 */
export function computePathLatency(
  stages: NodeKind[],
  mode: Mode,
  params: LatencyParams = DEFAULT_LATENCY_PARAMS,
): PathLatency {
  const hops = Math.max(0, stages.length - 1);
  const propagationMs = hops * params.geoMsPerHop;
  const isMixnet = mode === "mixnet";
  const mixLayers = isMixnet ? stages.filter((s) => s === "mix").length : 0;
  const mixingMs = mixLayers * params.mixDelayMs;
  const ackMs = isMixnet ? hops * params.geoMsPerHop : 0;
  return {
    hops,
    mixLayers,
    propagationMs,
    mixingMs,
    ackMs,
    totalMs: propagationMs + mixingMs + ackMs,
  };
}

/** Human-readable rounding for display (ms → "1.2 s" past 1000 ms). */
export function formatLatency(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)} s`;
  return `${Math.round(ms)} ms`;
}
