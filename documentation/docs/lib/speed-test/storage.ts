// Stored speed-test runs and pending tunnel options, kept in sessionStorage so
// a comparison survives the reload that the one-shot tunnel needs. Every access
// can throw (private mode, blocked storage), so each one is wrapped and reports
// failure instead of breaking the page. Call these only from effects or event
// handlers: the playground also renders on the server.

import type { Probe, StepSummary } from "./ramp";

const KEY = "nym-playground-speed-test-v1";
const MAX_RUNS = 6;

export interface TunnelConfig {
  disablePoissonTraffic: boolean;
  disableCoverTraffic: boolean;
}

export interface RampRun {
  kind: "ramp";
  config: TunnelConfig;
  payloadBytes: number;
  steps: StepSummary[];
  probes: Probe[];
  stopReason: string;
  finishedAt: string;
}

export interface HistogramRun {
  kind: "histogram";
  config: TunnelConfig;
  rtts: number[];
  finishedAt: string;
}

export type StoredRun = RampRun | HistogramRun;

export interface Stored {
  v: 1;
  runs: StoredRun[];
  pending: TunnelConfig | null;
}

const empty = (): Stored => ({ v: 1, runs: [], pending: null });

/** The stored state, or null when storage is not available. */
export function load(): Stored | null {
  let raw: string | null;
  try {
    raw = window.sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
  if (!raw) return empty();
  let parsed: Partial<Stored> | null;
  try {
    parsed = JSON.parse(raw);
  } catch {
    return empty(); // a corrupt value is dropped; storage itself still works
  }
  // An unknown schema is dropped rather than half-read.
  if (!parsed || parsed.v !== 1) return empty();
  return { v: 1, runs: Array.isArray(parsed.runs) ? parsed.runs : [], pending: parsed.pending ?? null };
}

/** Write the state, keeping the newest runs; false when storage is not available. */
export function save(state: Stored): boolean {
  try {
    window.sessionStorage.setItem(KEY, JSON.stringify({ ...state, runs: state.runs.slice(-MAX_RUNS) }));
    return true;
  } catch {
    return false;
  }
}

export function clear(): boolean {
  try {
    window.sessionStorage.removeItem(KEY);
    return true;
  } catch {
    return false;
  }
}

export function describeConfig(c: TunnelConfig): string {
  const poisson = c.disablePoissonTraffic ? 'Poisson off' : 'Poisson on';
  return c.disableCoverTraffic ? `${poisson}, loop cover off` : poisson;
}
