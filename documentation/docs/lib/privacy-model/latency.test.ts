import { describe, expect, it } from "vitest";
import {
  computePathLatency,
  DEFAULT_LATENCY_PARAMS,
  formatLatency,
} from "./latency";
import type { NodeKind } from "./types";

const MIXNET_PATH: NodeKind[] = [
  "client",
  "entry",
  "mix",
  "mix",
  "mix",
  "exit",
  "destination",
];
const DVPN_PATH: NodeKind[] = ["client", "entry", "exit", "destination"];

describe("computePathLatency", () => {
  it("adds mixing delay only for mixnet paths", () => {
    const mix = computePathLatency(MIXNET_PATH, "mixnet");
    const dvpn = computePathLatency(DVPN_PATH, "dvpn");
    expect(mix.mixingMs).toBeGreaterThan(0);
    expect(dvpn.mixingMs).toBe(0);
    expect(mix.mixLayers).toBe(3);
  });

  it("mixnet total exceeds dVPN total for equal geography", () => {
    const params = { geoMsPerHop: 120, mixDelayMs: 50 };
    const mix = computePathLatency(MIXNET_PATH, "mixnet", params);
    const dvpn = computePathLatency(DVPN_PATH, "dvpn", params);
    expect(mix.totalMs).toBeGreaterThan(dvpn.totalMs);
  });

  it("adds a reliable-channel ACK (geographic only) for mixnet, not dVPN", () => {
    const params = { geoMsPerHop: 120, mixDelayMs: 50 };
    const mix = computePathLatency(MIXNET_PATH, "mixnet", params);
    const dvpn = computePathLatency(DVPN_PATH, "dvpn", params);
    // ACK follows the original path back: hops × geo, no mixing.
    expect(mix.ackMs).toBe(mix.hops * params.geoMsPerHop);
    expect(dvpn.ackMs).toBe(0);
    expect(mix.totalMs).toBe(mix.propagationMs + mix.mixingMs + mix.ackMs);
  });

  it("mixnet total exceeds dVPN even on the same node path", () => {
    // Same geography, same stages: mixing delay is the only differentiator.
    const params = { geoMsPerHop: 120, mixDelayMs: 50 };
    const asMix = computePathLatency(MIXNET_PATH, "mixnet", params);
    const asDvpn = computePathLatency(MIXNET_PATH, "dvpn", params);
    expect(asMix.totalMs).toBeGreaterThan(asDvpn.totalMs);
    expect(asMix.propagationMs).toBe(asDvpn.propagationMs);
  });

  it("responds to parameter changes", () => {
    const base = computePathLatency(MIXNET_PATH, "mixnet", {
      geoMsPerHop: 40,
      mixDelayMs: 20,
    });
    const slower = computePathLatency(MIXNET_PATH, "mixnet", {
      geoMsPerHop: 300,
      mixDelayMs: 200,
    });
    expect(slower.totalMs).toBeGreaterThan(base.totalMs);
    expect(slower.propagationMs).toBeGreaterThan(base.propagationMs);
    expect(slower.mixingMs).toBeGreaterThan(base.mixingMs);
  });

  it("propagation scales with hop count", () => {
    const p = DEFAULT_LATENCY_PARAMS;
    const l = computePathLatency(MIXNET_PATH, "mixnet", p);
    expect(l.hops).toBe(MIXNET_PATH.length - 1);
    expect(l.propagationMs).toBe(l.hops * p.geoMsPerHop);
  });

  it("handles a trivial single-node path without error", () => {
    const l = computePathLatency(["client"], "direct");
    expect(l.hops).toBe(0);
    expect(l.totalMs).toBe(0);
  });
});

describe("formatLatency", () => {
  it("formats ms and seconds", () => {
    expect(formatLatency(250)).toBe("250 ms");
    expect(formatLatency(1500)).toBe("1.5 s");
  });
});
