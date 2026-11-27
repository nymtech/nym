// Model of the Nym client send loop and the mixnet round trip, for the
// playground speed test. Pure functions only: the simulator and the live-test
// overlays both use this module, and vitest covers it.

import { DEFAULT_MIX_DELAY_MS, SEND_INTERVAL_MS } from "../privacy-model/latency";

// Mirrors of the client-core defaults (common/client-core/config-types/src/lib.rs).
export const MAIN_STREAM_MEAN_MS = SEND_INTERVAL_MS; // DEFAULT_MESSAGE_STREAM_AVERAGE_DELAY
export const LOOP_COVER_MEAN_MS = 200; // DEFAULT_LOOP_COVER_STREAM_AVERAGE_DELAY
export const HOP_DELAY_MEAN_MS = DEFAULT_MIX_DELAY_MS; // DEFAULT_AVERAGE_PACKET_DELAY
export const MIX_HOPS_PER_DIRECTION = 3;
// The IPR flushes its return buffer on this interval (BUFFER_TIMEOUT in
// common/ip-packet-requests/src/codec.rs).
export const IPR_FLUSH_MS = 20;
export const MAIN_STREAM_PKT_PER_S = 1000 / MAIN_STREAM_MEAN_MS;

export type Rng = () => number;

// Small seeded PRNG so a seed always gives the same picture (Math.random cannot
// be seeded, and sampling at render time would break hydration).
export function mulberry32(seed: number): Rng {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

// Inverse-transform sample of an exponential distribution. `1 - u` keeps the
// argument of log in (0, 1], so the result is finite.
export function sampleExp(rng: Rng, meanMs: number): number {
  return -meanMs * Math.log(1 - rng());
}

export type PacketKind = "real" | "cover" | "loop";

export interface ObservedPacket {
  t: number;
  kind: PacketKind;
}

export interface QueuePoint {
  t: number;
  depth: number;
}

export interface SimOptions {
  /** Times (ms, ascending) at which the app hands one packet to the client. */
  appSends: number[];
  poisson: boolean;
  loopCover: boolean;
  durationMs: number;
  seed: number;
}

export interface SimResult {
  /** Packets the entry gateway receives, in time order. */
  packets: ObservedPacket[];
  /** Queue depth after each main-stream tick (empty when Poisson is off). */
  queue: QueuePoint[];
  /** Mean time a real packet waited in the queue, or null if none was sent. */
  meanWaitMs: number | null;
  /** App packets still queued at the end of the window. */
  unsent: number;
}

// Discrete-event model of the client send loop (real_traffic_stream.rs):
// - Poisson on: on each exponential tick, send one queued real packet, or a
//   cover packet if the queue is empty (poll_poisson).
// - Poisson off: each real packet leaves when it arrives; no main-stream cover
//   (poll_immediate).
// - The loop cover stream runs on its own exponential clock unless it is off.
// The two clocks use separate seeded streams, so turning one stream off does
// not move the ticks of the other. Gateway backpressure and retransmissions
// are not modelled.
export function simulateClient(opts: SimOptions): SimResult {
  const { appSends, poisson, loopCover, durationMs, seed } = opts;
  const packets: ObservedPacket[] = [];
  const queue: QueuePoint[] = [];

  if (loopCover) {
    const rng = mulberry32(seed + 1);
    for (let t = sampleExp(rng, LOOP_COVER_MEAN_MS); t < durationMs; t += sampleExp(rng, LOOP_COVER_MEAN_MS)) {
      packets.push({ t, kind: "loop" });
    }
  }

  let totalWait = 0;
  let realSent = 0;
  let unsent = 0;

  if (poisson) {
    const rng = mulberry32(seed);
    const pending: number[] = [];
    let next = 0;
    for (let t = sampleExp(rng, MAIN_STREAM_MEAN_MS); t < durationMs; t += sampleExp(rng, MAIN_STREAM_MEAN_MS)) {
      while (next < appSends.length && appSends[next] <= t) pending.push(appSends[next++]);
      const arrival = pending.shift();
      if (arrival === undefined) {
        packets.push({ t, kind: "cover" });
      } else {
        packets.push({ t, kind: "real" });
        totalWait += t - arrival;
        realSent++;
      }
      queue.push({ t, depth: pending.length });
    }
    while (next < appSends.length && appSends[next] < durationMs) pending.push(appSends[next++]);
    unsent = pending.length;
  } else {
    for (const t of appSends) {
      if (t >= durationMs) break;
      packets.push({ t, kind: "real" });
      realSent++;
    }
  }

  packets.sort((a, b) => a.t - b.t);
  return { packets, queue, meanWaitMs: realSent ? totalWait / realSent : null, unsent };
}

/**
 * Mean gap between packets at the gateway when both streams are Poisson: the
 * merge of two Poisson processes is Poisson with the summed rate. Null when
 * Poisson is off, because the gaps then follow the app, not a distribution.
 */
export function expectedGapMeanMs(poisson: boolean, loopCover: boolean): number | null {
  if (!poisson) return null;
  const rate = 1 / MAIN_STREAM_MEAN_MS + (loopCover ? 1 / LOOP_COVER_MEAN_MS : 0);
  return 1 / rate;
}

/**
 * One sample of the model round trip, without the time on network links:
 * six exponential mix-hop delays, one uniform wait for the IPR's flush timer,
 * and one forward send-slot wait when Poisson is on. There is no return-leg
 * slot wait because nym-node runs its IPR with `disable_poisson_rate: true`.
 */
export function sampleRttMs(rng: Rng, poisson: boolean): number {
  let total = rng() * IPR_FLUSH_MS + (poisson ? sampleExp(rng, MAIN_STREAM_MEAN_MS) : 0);
  for (let i = 0; i < 2 * MIX_HOPS_PER_DIRECTION; i++) total += sampleExp(rng, HOP_DELAY_MEAN_MS);
  return total;
}

export function sampleRtts(n: number, seed: number, poisson: boolean): number[] {
  const rng = mulberry32(seed);
  return Array.from({ length: n }, () => sampleRttMs(rng, poisson));
}

export interface Bins {
  start: number;
  width: number;
  count: number;
}

/** Counts per bin. Values outside the range go into the first or last bin. */
export function histogram(values: number[], bins: Bins): number[] {
  const counts = new Array<number>(bins.count).fill(0);
  for (const v of values) {
    const i = Math.floor((v - bins.start) / bins.width);
    counts[Math.min(bins.count - 1, Math.max(0, i))]++;
  }
  return counts;
}

/** Quantile by nearest rank on a copy; NaN for an empty input. */
export function quantile(values: number[], q: number): number {
  if (values.length === 0) return NaN;
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.max(0, Math.ceil(q * sorted.length) - 1))];
}

export const median = (values: number[]) => quantile(values, 0.5);
