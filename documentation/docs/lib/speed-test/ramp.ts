// Probe framing and result analysis for the live speed tests.

import { MAIN_STREAM_PKT_PER_S, median } from "./model";

// Each probe frame starts with a magic tag and a sequence number, so the
// handler can match echoes by number and ignore any other frame the server
// sends (for example a greeting on open).
const MAGIC = 0x4e594d53; // "NYMS"
export const HEADER_BYTES = 8;

export function encodeProbe(seq: number, size: number): Uint8Array {
  const buf = new Uint8Array(Math.max(HEADER_BYTES, size));
  // Random body, so no compression on the path can shrink the probe.
  crypto.getRandomValues(buf.subarray(HEADER_BYTES));
  const view = new DataView(buf.buffer);
  view.setUint32(0, MAGIC);
  view.setUint32(4, seq);
  return buf;
}

/** The sequence number of a probe frame, or null for any other frame. */
export function decodeProbe(data: unknown): number | null {
  if (!(data instanceof ArrayBuffer) || data.byteLength < HEADER_BYTES) return null;
  const view = new DataView(data);
  return view.getUint32(0) === MAGIC ? view.getUint32(4) : null;
}

// About 2 KB of payload per Sphinx packet at 50 packets per second, before any
// header or SURB takes its share.
export const SPHINX_PAYLOAD_BYTES = 2048;
export const RAW_BUDGET_KBPS = (MAIN_STREAM_PKT_PER_S * SPHINX_PAYLOAD_BYTES) / 1024;
// smolmix caps the TCP buffer, and so the receive window, at u16::MAX
// (tcp_connect in wasm/smolmix/src/stream.rs).
export const TCP_WINDOW_BYTES = 65535;
// A step whose RTT grows by more than 0.1 ms per ms of run time is over capacity.
export const KNEE_SLOPE = 0.1;

export interface Probe {
  step: number;
  /** ms since the step started. */
  sentAt: number;
  /** null when the echo did not arrive before the drain timeout. */
  rttMs: number | null;
}

export interface StepSummary {
  offeredKBps: number;
  sent: number;
  lost: number;
  medianRttMs: number;
  /** Least-squares slope of RTT against send time (ms per ms). */
  slope: number;
}

/** Least-squares slope of y on x; 0 with fewer than two points or no spread in x. */
export function fitSlope(points: { x: number; y: number }[]): number {
  const n = points.length;
  if (n < 2) return 0;
  const mx = points.reduce((a, p) => a + p.x, 0) / n;
  const my = points.reduce((a, p) => a + p.y, 0) / n;
  let sxy = 0;
  let sxx = 0;
  for (const p of points) {
    sxy += (p.x - mx) * (p.y - my);
    sxx += (p.x - mx) ** 2;
  }
  return sxx === 0 ? 0 : sxy / sxx;
}

export function summariseStep(offeredKBps: number, probes: Probe[]): StepSummary {
  const received = probes.filter((p) => p.rttMs !== null) as (Probe & { rttMs: number })[];
  return {
    offeredKBps,
    sent: probes.length,
    lost: probes.length - received.length,
    medianRttMs: median(received.map((p) => p.rttMs)),
    slope: fitSlope(received.map((p) => ({ x: p.sentAt, y: p.rttMs }))),
  };
}

/** Index of the first step over capacity, or null if no step is. */
export function findKnee(steps: StepSummary[]): number | null {
  const i = steps.findIndex((s) => s.slope > KNEE_SLOPE);
  return i === -1 ? null : i;
}

/**
 * Fluid-queue estimate of the path capacity. If the path carries mu and the
 * offered load is lambda > mu, the queue grows at lambda - mu and the RTT slope
 * is lambda / mu - 1, so mu = lambda / (1 + slope).
 */
export function capacityKBps(step: StepSummary): number {
  return step.offeredKBps / (1 + step.slope);
}

/** Throughput that a full TCP window allows at the given RTT. */
export function windowLimitKBps(rttMs: number): number {
  return TCP_WINDOW_BYTES / 1024 / (rttMs / 1000);
}
