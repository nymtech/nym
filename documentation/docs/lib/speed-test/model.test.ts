import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  expectedGapMeanMs,
  histogram,
  HOP_DELAY_MEAN_MS,
  IPR_FLUSH_MS,
  LOOP_COVER_MEAN_MS,
  MAIN_STREAM_MEAN_MS,
  median,
  MIX_HOPS_PER_DIRECTION,
  mulberry32,
  quantile,
  sampleExp,
  sampleRtts,
  simulateClient,
} from "./model";
import { capacityKBps, decodeProbe, encodeProbe, findKnee, summariseStep, windowLimitKBps, type Probe } from "./ramp";

const WINDOW_MS = 10_000;
const burst = (n: number, at: number) => new Array<number>(n).fill(at);
const stream = (perSecond: number, durationMs: number) =>
  Array.from({ length: Math.floor((perSecond * durationMs) / 1000) }, (_, i) => (i * 1000) / perSecond);

// Read the defaults from the Rust sources, so a change there fails this test.
const REPO = path.resolve(__dirname, "../../../..");
function rustMillis(file: string, name: string): number {
  const src = fs.readFileSync(path.join(REPO, file), "utf8");
  const m = src.match(new RegExp(`${name}:\\s*Duration\\s*=\\s*Duration::from_millis\\((\\d+)\\)`));
  if (!m) throw new Error(`${name} not found in ${file}`);
  return Number(m[1]);
}

describe("model constants match the Rust sources", () => {
  const CLIENT = "common/client-core/config-types/src/lib.rs";
  it.each([
    ["DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY", CLIENT, MAIN_STREAM_MEAN_MS],
    ["DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY", CLIENT, LOOP_COVER_MEAN_MS],
    ["DEFAULT_AVERAGE_PACKET_DELAY", CLIENT, HOP_DELAY_MEAN_MS],
    ["BUFFER_TIMEOUT", "common/ip-packet-requests/src/codec.rs", IPR_FLUSH_MS],
  ])("%s", (name, file, value) => {
    expect(value).toBe(rustMillis(file, name));
  });

  it("uses three mix hops in each direction", () => {
    expect(MIX_HOPS_PER_DIRECTION).toBe(3);
  });
});

describe("sampling", () => {
  it("gives the same sequence for the same seed", () => {
    const a = mulberry32(7);
    const b = mulberry32(7);
    for (let i = 0; i < 100; i++) expect(a()).toBe(b());
  });

  it("sampleExp has the requested mean", () => {
    const rng = mulberry32(1);
    const n = 100_000;
    let sum = 0;
    for (let i = 0; i < n; i++) sum += sampleExp(rng, 20);
    expect(sum / n).toBeGreaterThan(19.5);
    expect(sum / n).toBeLessThan(20.5);
  });
});

describe("simulateClient", () => {
  const base = { durationMs: WINDOW_MS, seed: 42, loopCover: true };

  it("is deterministic for a seed", () => {
    const a = simulateClient({ ...base, appSends: burst(40, 3000), poisson: true });
    const b = simulateClient({ ...base, appSends: burst(40, 3000), poisson: true });
    expect(a).toEqual(b);
  });

  it("with Poisson on, idle and burst give the same packet times", () => {
    const idle = simulateClient({ ...base, appSends: [], poisson: true });
    const busy = simulateClient({ ...base, appSends: burst(40, 3000), poisson: true });
    expect(busy.packets.map((p) => p.t)).toEqual(idle.packets.map((p) => p.t));
    expect(busy.packets.filter((p) => p.kind === "real")).toHaveLength(40);
  });

  it("with Poisson on and loop cover off, empty ticks are still filled with cover", () => {
    const r = simulateClient({ ...base, loopCover: false, appSends: [], poisson: true });
    const rate = r.packets.length / (WINDOW_MS / 1000);
    expect(r.packets.every((p) => p.kind === "cover")).toBe(true);
    expect(rate).toBeGreaterThan(40);
    expect(rate).toBeLessThan(60);
  });

  it("with Poisson off, the main stream sends no cover and real packets leave at once", () => {
    const r = simulateClient({ ...base, appSends: burst(40, 3000), poisson: false });
    expect(r.packets.some((p) => p.kind === "cover")).toBe(false);
    expect(r.packets.filter((p) => p.kind === "real").every((p) => p.t === 3000)).toBe(true);
    expect(r.queue).toHaveLength(0);
  });

  it("an over-budget stream builds a queue that grows", () => {
    const r = simulateClient({ ...base, appSends: stream(80, WINDOW_MS), poisson: true });
    const early = r.queue.filter((q) => q.t < 2000).map((q) => q.depth);
    const late = r.queue.filter((q) => q.t > 8000).map((q) => q.depth);
    expect(Math.max(...late)).toBeGreaterThan(Math.max(...early));
    expect(r.unsent).toBeGreaterThan(100);
  });

  it("gap mean at the gateway matches the merged Poisson rate", () => {
    const r = simulateClient({ ...base, durationMs: 200_000, appSends: [], poisson: true });
    const gaps = r.packets.slice(1).map((p, i) => p.t - r.packets[i].t);
    const mean = gaps.reduce((a, b) => a + b, 0) / gaps.length;
    const expected = expectedGapMeanMs(true, true) as number;
    expect(Math.abs(mean - expected) / expected).toBeLessThan(0.03);
  });
});

describe("RTT model", () => {
  // Six hops of 15 ms, a 10 ms mean flush wait, and a 20 ms slot wait.
  it("has the mean of hop delays, flush wait and one slot wait", () => {
    const rtts = sampleRtts(50_000, 3, true);
    const mean = rtts.reduce((a, b) => a + b, 0) / rtts.length;
    expect(mean).toBeGreaterThan(118);
    expect(mean).toBeLessThan(122);
  });

  it("drops only the slot wait with Poisson off", () => {
    const rtts = sampleRtts(50_000, 3, false);
    const mean = rtts.reduce((a, b) => a + b, 0) / rtts.length;
    expect(mean).toBeGreaterThan(98);
    expect(mean).toBeLessThan(102);
  });
});

describe("stats helpers", () => {
  it("histogram clamps outliers into the edge bins", () => {
    expect(histogram([-5, 0, 9, 10, 99], { start: 0, width: 10, count: 3 })).toEqual([3, 1, 1]);
  });

  it("quantile uses nearest rank", () => {
    expect(median([3, 1, 2])).toBe(2);
    expect(quantile([1, 2, 3, 4], 0.95)).toBe(4);
    expect(Number.isNaN(median([]))).toBe(true);
  });
});

describe("probe framing", () => {
  it("round-trips the sequence number", () => {
    const buf = encodeProbe(1234, 64);
    expect(buf.byteLength).toBe(64);
    expect(decodeProbe(buf.buffer)).toBe(1234);
  });

  it("ignores text frames and frames without the tag", () => {
    expect(decodeProbe("Request served by abc")).toBeNull();
    expect(decodeProbe(new Uint8Array(16).buffer)).toBeNull();
    expect(decodeProbe(new Uint8Array(4).buffer)).toBeNull();
  });
});

describe("ramp analysis", () => {
  const probes = (rtt: (sentAt: number) => number | null, n = 40, stepMs = 5000): Probe[] =>
    Array.from({ length: n }, (_, i) => {
      const sentAt = (i * stepMs) / n;
      return { step: 0, sentAt, rttMs: rtt(sentAt) };
    });

  it("fits a flat step to slope 0 and a climbing step to its slope", () => {
    expect(summariseStep(16, probes(() => 600)).slope).toBeCloseTo(0);
    expect(summariseStep(96, probes((t) => 600 + 0.5 * t)).slope).toBeCloseTo(0.5);
  });

  it("counts lost echoes and leaves them out of the fit", () => {
    const s = summariseStep(16, probes((t) => (Math.round(t) % 500 === 0 ? null : 600)));
    expect(s.lost).toBe(10);
    expect(s.sent).toBe(40);
    expect(s.slope).toBeCloseTo(0);
  });

  it("finds the knee by slope and estimates capacity from it", () => {
    const steps = [
      summariseStep(32, probes(() => 600)),
      summariseStep(64, probes((t) => 600 + 0.05 * t)),
      summariseStep(96, probes((t) => 600 + 0.5 * t)),
    ];
    const knee = findKnee(steps);
    expect(knee).toBe(2);
    expect(capacityKBps(steps[knee as number])).toBeCloseTo(64);
  });

  it("reports no knee when every step is flat", () => {
    expect(findKnee([summariseStep(8, probes(() => 600))])).toBeNull();
  });

  it("window limit is 64 KiB per RTT", () => {
    expect(windowLimitKBps(1000)).toBeCloseTo(64, 0);
  });
});
