"use client";

// Live speed tests on the playground tunnel: a throughput ramp (where does the
// RTT start to climb?) and an RTT histogram (how wide is the deliberate spread?).
// The maths is in lib/speed-test; this file runs the probes and draws results.
//
// Timing rules: the message handler takes performance.now() before anything
// else, results go into refs, and the charts re-render at 4 Hz while a run is
// in progress. A render can still delay an event by a few ms, which is small
// next to RTTs of hundreds of ms.

import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { scaleLinear } from 'd3-scale';
import { histogram, quantile, sampleRtts, type Bins } from '../../lib/speed-test/model';
import {
  capacityKBps,
  decodeProbe,
  encodeProbe,
  findKnee,
  RAW_BUDGET_KBPS,
  summariseStep,
  windowLimitKBps,
  type Probe,
  type StepSummary,
} from '../../lib/speed-test/ramp';
import * as storage from '../../lib/speed-test/storage';
import type { HistogramRun, RampRun, StoredRun, TunnelConfig } from '../../lib/speed-test/storage';
import type { MixWebSocketLike, PlaygroundMods } from './lib';
import { Button, LogPanel, StatusText, box, input, legend, num, row, sub, type Colour, type LogEntry, type Status } from './ui';
import { BinTargets, Legend, SERIES, Tooltip, VIZ_CLASS, VizStyle, XAxis, YAxis, innerHeight, svgPoint, type Frame } from './chart';

const RAMP_STEPS_KBPS = [8, 16, 32, 48, 64, 96, 128];
const STEP_MS = 5_000;
const DRAIN_MS = 30_000;
const STOP_MEDIAN_MS = 5_000;
const OPEN_TIMEOUT_MS = 30_000;
const HIST_PAYLOAD = 32;
const MIN_HIST_RTTS = 10;
const MAX_CHART_RUNS = 3;

// NaN goes through JSON as null, so stored runs can hold either.
const fmtMs = (v: number | null) => (v !== null && Number.isFinite(v) ? v.toFixed(0) : '-');

/** Integer from a text field, clamped to [lo, hi]; lo for empty or bad input. */
function clampInt(text: string, lo: number, hi: number): number {
  const n = Math.round(Number(text));
  return Number.isFinite(n) ? Math.min(hi, Math.max(lo, n)) : lo;
}

// Colour follows the configuration, so a new run never repaints the old ones.
function colourFor(c: TunnelConfig): string {
  if (c.disableCoverTraffic) return SERIES[2];
  return c.disablePoissonTraffic ? SERIES[1] : SERIES[0];
}

const runLabel = (c: TunnelConfig, finishedAt?: string) =>
  `${storage.describeConfig(c)}, ${finishedAt ? new Date(finishedAt).toLocaleTimeString() : 'running'}`;

type Phase = 'idle' | 'connecting' | 'running' | 'draining';

interface Outstanding {
  sentAt: number;
  /** Settles the probe: an RTT, or null when it is lost or aborted. */
  done: (rttMs: number | null) => void;
}

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, Math.max(0, ms)));

export interface SpeedTestSectionProps {
  mods: PlaygroundMods | null;
  /** Options that setupMixTunnel received, or null before setup. */
  activeConfig: TunnelConfig | null;
  /** Another playground section is sending traffic. */
  otherBusy: boolean;
  setSpeedTestBusy: (busy: boolean) => void;
  log: (section: string, msg: string, colour?: Colour) => void;
  lines: (section: string) => LogEntry[];
}

export function SpeedTestSection({ mods, activeConfig, otherBusy, setSpeedTestBusy, log, lines }: SpeedTestSectionProps) {
  const [url, setUrl] = useState('wss://echo.websocket.org');
  // Raw text, so a value can be typed digit by digit; clamped where it is used.
  const [payloadText, setPayloadText] = useState('1024');
  const [histCountText, setHistCountText] = useState('100');
  const payload = clampInt(payloadText, 1024, 16384);
  const histCount = clampInt(histCountText, 20, 500);
  const [phase, setPhase] = useState<Phase>('idle');
  const [status, setStatus] = useState<Status>({ text: '' });
  const [runs, setRuns] = useState<StoredRun[]>([]);
  const [storageOk, setStorageOk] = useState(true);
  const [, setTick] = useState(0);

  const socket = useRef<MixWebSocketLike | null>(null);
  const socketUrl = useRef('');
  const outstanding = useRef(new Map<number, Outstanding>());
  const nextSeq = useRef(1);
  const inFlight = useRef(false);
  const stopReason = useRef<string | null>(null);
  const liveRamp = useRef<{ probes: Probe[]; steps: StepSummary[] } | null>(null);
  const liveRtts = useRef<number[] | null>(null);

  // Storage is read only after mount: this component also renders on the server.
  useEffect(() => {
    const s = storage.load();
    if (s) setRuns(s.runs);
    else setStorageOk(false);
  }, []);

  // Set the stop reason (the first one wins) and settle every probe in flight,
  // so a run ends at once and does not wait for the drain timeout.
  const stop = useCallback((reason: string) => {
    stopReason.current ??= reason;
    const pending = [...outstanding.current.values()];
    outstanding.current.clear();
    for (const p of pending) p.done(null);
  }, []);

  // A hidden tab throttles timers, so the schedule would turn into bursts.
  useEffect(() => {
    const onVisibility = () => {
      if (document.hidden) stop('the tab was hidden, so the browser slowed the timers');
    };
    document.addEventListener('visibilitychange', onVisibility);
    return () => document.removeEventListener('visibilitychange', onVisibility);
  }, [stop]);

  // Re-render at 4 Hz while a run is in progress.
  const running = phase === 'running' || phase === 'draining';
  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setTick((t) => t + 1), 250);
    return () => clearInterval(id);
  }, [running]);

  useEffect(
    () => () => {
      stop('the section was unmounted');
      socket.current?.close().catch(() => {});
    },
    [stop],
  );

  const onMessage = useCallback((ev: Event) => {
    const now = performance.now();
    const seq = decodeProbe((ev as MessageEvent).data);
    if (seq === null) return;
    const entry = outstanding.current.get(seq);
    if (!entry) return; // late echo from a cancelled or drained run
    entry.done(now - entry.sentAt);
  }, []);

  async function ensureSocket(): Promise<MixWebSocketLike> {
    const target = url.trim();
    if (socket.current && socket.current.readyState === 1 && socketUrl.current === target) return socket.current;
    if (!mods) throw new Error('tunnel not set up');
    // Detach the old socket first, so its close event does not stop this run.
    const old = socket.current;
    socket.current = null;
    await old?.close().catch(() => {});
    setPhase('connecting');
    log('speed', `connecting to ${target}`);
    const ws = new mods.MixWebSocket(target);
    // Set before the await, so an unmount while connecting still closes it.
    socket.current = ws;
    socketUrl.current = target;
    ws.addEventListener('message', onMessage);
    // MixWebSocket dispatches 'error' without 'close' when the stream fails.
    const onLost = () => {
      if (socket.current !== ws) return;
      socket.current = null;
      stop('the WebSocket closed');
    };
    ws.addEventListener('close', onLost);
    ws.addEventListener('error', onLost);
    // opened() also resolves on error, and it has no timeout of its own.
    await Promise.race([ws.opened(), sleep(OPEN_TIMEOUT_MS)]);
    if (ws.readyState !== 1) {
      if (socket.current === ws) socket.current = null;
      ws.close().catch(() => {});
      throw new Error(`could not open ${target}`);
    }
    log('speed', 'connected', 'green');
    return ws;
  }

  /** Send one probe; resolves with its RTT, or null when lost, aborted or after `timeoutMs`. */
  function sendProbe(ws: MixWebSocketLike, size: number, timeoutMs: number): Promise<number | null> {
    const seq = nextSeq.current++;
    return new Promise((resolve) => {
      const done = (rtt: number | null) => {
        clearTimeout(timer);
        outstanding.current.delete(seq);
        resolve(rtt);
      };
      const timer = setTimeout(() => done(null), timeoutMs);
      outstanding.current.set(seq, { sentAt: performance.now(), done });
      ws.send(encodeProbe(seq, size)).catch((e) => {
        log('speed', `send failed: ${e}`, 'red');
        stop('a send failed');
      });
    });
  }

  function finishRun(run: StoredRun) {
    setRuns((rs) => [...rs, run]);
    const s = storage.load();
    setStorageOk(s !== null && storage.save({ ...s, runs: [...s.runs, run] }));
  }

  async function guarded(name: string, body: (ws: MixWebSocketLike, config: TunnelConfig) => Promise<void>) {
    // A ref, not `phase`: phase changes only after an await, so a second click
    // in that window would start a second run.
    if (!activeConfig || otherBusy || inFlight.current) return;
    inFlight.current = true;
    stopReason.current = null;
    setSpeedTestBusy(true);
    setStatus({ text: 'Running...', colour: 'orange' });
    try {
      const ws = await ensureSocket();
      setPhase('running');
      await body(ws, activeConfig);
    } catch (e) {
      log('speed', `${name} failed: ${e}`, 'red');
      setStatus({ text: `Failed: ${e}`, colour: 'red' });
    } finally {
      inFlight.current = false;
      setPhase('idle');
      setSpeedTestBusy(false);
    }
  }

  const runRamp = () =>
    guarded('ramp', async (ws, config) => {
      const steps: StepSummary[] = [];
      const probes: Probe[] = [];
      liveRamp.current = { probes, steps };
      let why = 'all steps done';
      log('speed', `ramp: ${payload} B messages, ${RAMP_STEPS_KBPS.join(', ')} KB/s, ${STEP_MS / 1000} s each (${storage.describeConfig(config)})`);

      for (let k = 0; k < RAMP_STEPS_KBPS.length; k++) {
        const rate = RAMP_STEPS_KBPS[k];
        const intervalMs = (payload / (rate * 1024)) * 1000;
        const count = Math.floor(STEP_MS / intervalMs);
        const stepProbes: Probe[] = [];
        const echoes: Promise<void>[] = [];
        setPhase('running');
        const stepStart = performance.now();
        for (let i = 0; i < count && !stopReason.current; i++) {
          // Each send time comes from the step start, so timer drift does not build up.
          await sleep(stepStart + i * intervalMs - performance.now());
          const probe: Probe = { step: k, sentAt: performance.now() - stepStart, rttMs: null };
          stepProbes.push(probe);
          probes.push(probe);
          echoes.push(sendProbe(ws, payload, DRAIN_MS).then((rtt) => void (probe.rttMs = rtt)));
        }
        setPhase('draining');
        await Promise.all(echoes);
        if (stopReason.current) {
          // A stopped step has no summary, so its probes go too.
          probes.splice(probes.length - stepProbes.length);
          why = stopReason.current;
          break;
        }

        const summary = summariseStep(rate, stepProbes);
        steps.push(summary);
        log(
          'speed',
          `${rate} KB/s: median RTT ${fmtMs(summary.medianRttMs)} ms, slope ${summary.slope.toFixed(2)}, lost ${summary.lost}/${summary.sent}`,
          summary.slope > 0.1 ? 'orange' : undefined,
        );
        const knee = findKnee(steps);
        if (knee !== null && k > knee) { why = 'one step past the knee'; break; }
        if (summary.lost > summary.sent / 2) { why = 'more than half of the step was lost'; break; }
        if (summary.medianRttMs > STOP_MEDIAN_MS) { why = `the median RTT passed ${STOP_MEDIAN_MS / 1000} s`; break; }
      }

      liveRamp.current = null;
      if (steps.length === 0) {
        log('speed', `ramp stopped before the first step finished: ${why}. Nothing was stored.`, 'orange');
        setStatus({ text: `Ramp stopped (${why})`, colour: 'orange' });
        return;
      }
      finishRun({ kind: 'ramp', config, payloadBytes: payload, steps, probes, stopReason: why, finishedAt: new Date().toISOString() });
      log('speed', `ramp stopped: ${why}`, 'green');
      setStatus({ text: `Ramp done (${why})`, colour: 'green' });
    });

  const runHistogram = () =>
    guarded('histogram', async (ws, config) => {
      const rtts: number[] = [];
      liveRtts.current = rtts;
      let lost = 0;
      log('speed', `histogram: ${histCount} echoes of ${HIST_PAYLOAD} B, one at a time (${storage.describeConfig(config)})`);
      for (let i = 0; i < histCount && !stopReason.current; i++) {
        const rtt = await sendProbe(ws, HIST_PAYLOAD, DRAIN_MS);
        if (rtt === null) lost++;
        else rtts.push(rtt);
        // At least 100 ms clears the client's delayed TCP ACK; the random part
        // avoids phase-lock with the IPR's 20 ms flush timer.
        await sleep(100 + Math.random() * 200);
      }
      liveRtts.current = null;
      const why = stopReason.current ?? `${rtts.length} echoes, ${lost} lost`;
      if (rtts.length < MIN_HIST_RTTS) {
        log('speed', `histogram stopped with ${rtts.length} echoes: ${why}. Nothing was stored.`, 'orange');
        setStatus({ text: `Histogram stopped (${why})`, colour: 'orange' });
        return;
      }
      finishRun({ kind: 'histogram', config, rtts, finishedAt: new Date().toISOString() });
      log('speed', `histogram done: ${why}`, 'green');
      setStatus({ text: `Histogram done (${why})`, colour: 'green' });
    });

  function reloadWith(disablePoissonTraffic: boolean) {
    const s = storage.load();
    const pending = { disablePoissonTraffic, disableCoverTraffic: activeConfig?.disableCoverTraffic ?? false };
    if (!s || !storage.save({ ...s, pending })) {
      setStorageOk(false);
      return;
    }
    window.location.reload();
  }

  function clearRuns() {
    setStorageOk(storage.clear());
    setRuns([]);
  }

  const rampRuns = runs.filter((r): r is RampRun => r.kind === 'ramp').slice(-MAX_CHART_RUNS);
  const histRuns = runs.filter((r): r is HistogramRun => r.kind === 'histogram').slice(-MAX_CHART_RUNS);
  const live = liveRamp.current;
  const shownRamps: ShownRamp[] = [
    ...rampRuns.map((r) => ({ label: runLabel(r.config, r.finishedAt), colour: colourFor(r.config), probes: r.probes, steps: r.steps })),
    ...(live && activeConfig ? [{ label: runLabel(activeConfig), colour: colourFor(activeConfig), probes: live.probes, steps: live.steps }] : []),
  ].slice(-MAX_CHART_RUNS);
  const canStart = !!activeConfig && !otherBusy && !running && phase !== 'connecting';
  const poissonOff = activeConfig?.disablePoissonTraffic ?? false;

  return (
    <div className={VIZ_CLASS} style={box}>
      <VizStyle />
      <div style={legend}>Speed test</div>
      <div style={sub}>
        Both tests send echo messages over their own WebSocket in the tunnel. While a test runs, the other sections are paused, because all traffic shares one send queue.
      </div>
      <div style={{ ...row, marginTop: '0.5rem' }}>
        <label style={sub}>Echo server</label>
        <input style={input} value={url} onChange={(e) => setUrl(e.target.value)} disabled={running} />
      </div>
      <div style={row}>
        <label style={sub}>Ramp message size (bytes)</label>
        <input style={num} type="number" min={1024} max={16384} value={payloadText} onChange={(e) => setPayloadText(e.target.value)} onBlur={() => setPayloadText(String(payload))} disabled={running} />
        <Button onClick={runRamp} disabled={!canStart}>Run throughput ramp</Button>
      </div>
      <div style={row}>
        <label style={sub}>Histogram echoes</label>
        <input style={num} type="number" min={20} max={500} value={histCountText} onChange={(e) => setHistCountText(e.target.value)} onBlur={() => setHistCountText(String(histCount))} disabled={running} />
        <Button onClick={runHistogram} disabled={!canStart}>Run RTT histogram</Button>
      </div>
      <div style={row}>
        <Button onClick={() => stop('cancelled')} disabled={phase === 'idle'}>Cancel</Button>
        {phase === 'draining' && <span style={sub}>Draining: waiting for the echoes still in flight.</span>}
        <StatusText status={status} />
      </div>
      {!activeConfig && <div style={sub}>Set up the tunnel first.</div>}
      {otherBusy && <div style={sub}>Another section is sending traffic. Wait until it finishes.</div>}

      <div style={row}>
        <Button onClick={() => reloadWith(!poissonOff)} disabled={!activeConfig || running || !storageOk}>
          {`Reload to run again with Poisson ${poissonOff ? 'on' : 'off'}`}
        </Button>
        <Button onClick={clearRuns} disabled={running || runs.length === 0}>Clear stored runs</Button>
      </div>
      <div style={sub}>
        The tunnel is one-shot, so a comparison needs a reload. The page keeps the results in this tab and fills in the option for you. Each reload makes a new client, which can use a different gateway.
      </div>
      {!storageOk && <div style={{ ...sub, color: '#dc2626' }}>Session storage is not available, so the page cannot keep runs for a comparison. The tests still work.</div>}

      {shownRamps.length > 0 && <RampResults runs={shownRamps} />}
      {(histRuns.length > 0 || liveRtts.current) && (
        <HistogramResults
          runs={[
            ...histRuns.map((r) => ({ label: runLabel(r.config, r.finishedAt), poisson: !r.config.disablePoissonTraffic, rtts: r.rtts })),
            ...(liveRtts.current && activeConfig
              ? [{ label: runLabel(activeConfig), poisson: !activeConfig.disablePoissonTraffic, rtts: liveRtts.current }]
              : []),
          ].slice(-MAX_CHART_RUNS)}
        />
      )}
      <LogPanel lines={lines('speed')} />
    </div>
  );
}

interface ShownRamp {
  label: string;
  colour: string;
  probes: Probe[];
  steps: StepSummary[];
}

function RampResults({ runs }: { runs: ShownRamp[] }) {
  const frame: Frame = { width: 720, height: 260, left: 48, right: 8, top: 24, bottom: 34 };
  const [hover, setHover] = useState<number | null>(null);
  const stepCount = Math.max(1, ...runs.map((r) => Math.max(r.steps.length, ...r.probes.map((p) => p.step + 1))));
  const maxRtt = Math.max(1000, ...runs.flatMap((r) => r.probes.map((p) => p.rttMs ?? 0)));
  const x = scaleLinear().domain([0, stepCount]).range([frame.left, frame.width - frame.right]);
  const y = scaleLinear().domain([0, maxRtt / 1000]).nice().range([frame.height - frame.bottom, frame.top]);
  const latest = runs[runs.length - 1];

  function onMove(e: React.PointerEvent<SVGRectElement>) {
    const k = Math.floor(x.invert(svgPoint(e, frame).x));
    setHover(k >= 0 && k < stepCount ? k : null);
  }

  return (
    <div style={{ marginTop: '0.75rem' }}>
      <div style={{ ...legend, marginBottom: 0 }}>Throughput ramp: RTT of each echo</div>
      <Legend items={runs.map((r) => ({ label: r.label, colour: r.colour }))} />
      <svg viewBox={`0 0 ${frame.width} ${frame.height}`} role="img" aria-label="RTT of each echo, by offered load">
        <YAxis scale={y} frame={frame} label="RTT (s)" />
        {Array.from({ length: stepCount }, (_, k) => (
          <g key={k}>
            {k > 0 && <line x1={x(k)} x2={x(k)} y1={frame.top} y2={frame.height - frame.bottom} stroke="var(--viz-grid)" />}
            <text x={x(k + 0.5)} y={frame.height - frame.bottom + 14} textAnchor="middle">{`${RAMP_STEPS_KBPS[k]}`}</text>
          </g>
        ))}
        <line x1={frame.left} x2={frame.width - frame.right} y1={y(0)} y2={y(0)} stroke="var(--viz-axis)" />
        <text x={frame.left + (frame.width - frame.left - frame.right) / 2} y={frame.height - 2} textAnchor="middle">
          Offered load (KB/s); each column is one 5 s step
        </text>
        {runs.map((r, i) =>
          r.probes.map((p, j) =>
            p.rttMs === null ? null : (
              <circle key={`${i}-${j}`} cx={x(p.step + p.sentAt / STEP_MS)} cy={y(p.rttMs / 1000)} r={1.8} fill={r.colour} />
            ),
          ),
        )}
        {hover !== null && (
          <Tooltip
            x={x(hover + 0.5)}
            y={frame.top + 40}
            frame={frame}
            lines={[
              `${RAMP_STEPS_KBPS[hover]} KB/s`,
              ...runs.map((r) => {
                const s = r.steps[hover];
                return s ? `${r.label}: median ${fmtMs(s.medianRttMs)} ms, slope ${s.slope.toFixed(2)}` : `${r.label}: not run`;
              }),
            ]}
          />
        )}
        <rect x={frame.left} y={frame.top} width={x(stepCount) - frame.left} height={innerHeight(frame)} fill="transparent" onPointerMove={onMove} onPointerLeave={() => setHover(null)} />
      </svg>
      {runs.map((r, i) => (
        <RampVerdict key={i} label={r.label} steps={r.steps} />
      ))}
      <StepTable steps={latest.steps} />
      <div style={sub}>
        The slope is the extra RTT per unit of run time. Above the limit the queue grows, so the RTT climbs during the step. The capacity estimate is offered / (1 + slope). It assumes a constant cost per byte in the step. Nagle merges small writes, which raises the flat RTT by up to one round trip but adds no slope. Gateway backpressure, TCP retransmits and a SURB shortage at the IPR can also lower the limit.
      </div>
    </div>
  );
}

function RampVerdict({ label, steps }: { label: string; steps: StepSummary[] }) {
  if (steps.length === 0) return null;
  const knee = findKnee(steps);
  const base = steps[0].medianRttMs;
  // A first step with no echoes has no base RTT (NaN, or null after storage).
  const windowNote =
    base !== null && Number.isFinite(base)
      ? ` The TCP window allows about ${windowLimitKBps(base).toFixed(0)} KB/s at the base RTT of ${base.toFixed(0)} ms.`
      : '';
  if (knee === null) {
    return (
      <div style={{ fontSize: 13, margin: '0.4rem 0' }}>
        <strong>{label}:</strong> no knee up to {steps[steps.length - 1].offeredKBps} KB/s.{windowNote}
      </div>
    );
  }
  const capacity = capacityKBps(steps[knee]);
  return (
    <div style={{ fontSize: 13, margin: '0.4rem 0' }}>
      <strong>{label}:</strong> knee at {steps[knee].offeredKBps} KB/s. Estimated capacity {capacity.toFixed(0)} KB/s, which is {((100 * capacity) / RAW_BUDGET_KBPS).toFixed(0)}% of the raw send budget of {RAW_BUDGET_KBPS.toFixed(0)} KB/s (50 packets per second of 2 KB). Headers, reply SURBs, framing and TCP ACKs use the rest.{windowNote}
    </div>
  );
}

function StepTable({ steps }: { steps: StepSummary[] }) {
  const cell: React.CSSProperties = { padding: '2px 10px 2px 0', fontVariantNumeric: 'tabular-nums', textAlign: 'right' };
  return (
    <table style={{ fontSize: 12, margin: '0.5rem 0', borderCollapse: 'collapse' }}>
      <thead>
        <tr style={{ opacity: 0.7 }}>
          <th style={cell}>Offered (KB/s)</th>
          <th style={cell}>Echoes</th>
          <th style={cell}>Lost</th>
          <th style={cell}>Median RTT (ms)</th>
          <th style={cell}>Slope</th>
        </tr>
      </thead>
      <tbody>
        {steps.map((s) => (
          <tr key={s.offeredKBps}>
            <td style={cell}>{s.offeredKBps}</td>
            <td style={cell}>{s.sent - s.lost}</td>
            <td style={cell}>{s.lost}</td>
            <td style={cell}>{fmtMs(s.medianRttMs)}</td>
            <td style={cell}>{s.slope.toFixed(2)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

const MODEL_SAMPLES = 20_000;

function HistogramResults({ runs: allRuns }: { runs: { label: string; poisson: boolean; rtts: number[] }[] }) {
  // The live run has no echoes until the first one returns.
  const runs = allRuns.filter((r) => r.rtts.length >= 2);
  const all = runs.flatMap((r) => r.rtts);
  if (all.length < 2) return null;
  // One set of bins for every run, so the charts can be compared.
  const lo = Math.floor(Math.min(...all) / 10) * 10;
  const hi = quantile(all, 0.99);
  const width = Math.max(10, Math.ceil((hi - lo) / 30 / 10) * 10);
  const bins: Bins = { start: lo, width, count: Math.ceil((hi - lo) / width) + 1 };

  return (
    <div style={{ marginTop: '0.75rem' }}>
      <div style={{ ...legend, marginBottom: 0 }}>RTT histogram</div>
      <Legend items={[{ label: 'Measured echoes', colour: SERIES[0] }, { label: 'Model: mixing and pacing delays only, shifted by the fitted offset', colour: SERIES[1], shape: 'line' }]} />
      {runs.map((r, i) => (
        <HistogramChart key={i} run={r} bins={bins} />
      ))}
      <div style={sub}>
        The model is six exponential hop delays (mean 15 ms), a wait of 0 to 20 ms for the IPR's flush timer and, with Poisson on, one wait for a send slot (mean 20 ms). The fitted offset is the fixed time on network links, at the IPR and at the echo server, matched at the 5th percentile. Each packet takes a new random route, so the link times change from echo to echo, and the measured spread should be the same as the model or wider. With Poisson off only the slot wait goes: the sender still puts the hop delays in each packet.
      </div>
    </div>
  );
}

function HistogramChart({ run, bins }: { run: { label: string; poisson: boolean; rtts: number[] }; bins: Bins }) {
  const frame: Frame = { width: 720, height: 170, left: 44, right: 8, top: 22, bottom: 30 };
  const [hover, setHover] = useState<number | null>(null);
  const counts = histogram(run.rtts, bins);
  const model = useMemo(() => {
    const samples = sampleRtts(MODEL_SAMPLES, 1, run.poisson);
    return { samples, p5: quantile(samples, 0.05) };
  }, [run.poisson]);
  const offset = quantile(run.rtts, 0.05) - model.p5;
  const expected = histogram(model.samples.map((v) => v + offset), bins).map((c) => (c / MODEL_SAMPLES) * run.rtts.length);

  const x = scaleLinear().domain([bins.start, bins.start + bins.width * bins.count]).range([frame.left, frame.width - frame.right]);
  const y = scaleLinear().domain([0, Math.max(1, ...counts, ...expected)]).nice().range([frame.height - frame.bottom, frame.top]);
  const curve = expected.map((v, i) => `${i ? 'L' : 'M'}${x(bins.start + (i + 0.5) * bins.width).toFixed(1)},${y(v).toFixed(1)}`).join('');
  const barW = Math.max(1, x(bins.start + bins.width) - x(bins.start) - 2);
  const stats = [0, 0.5, 0.95, 1].map((q) => quantile(run.rtts, q).toFixed(0));

  return (
    <div>
      <div style={{ fontSize: 13, marginTop: '0.5rem' }}>
        <strong>{run.label}:</strong> {run.rtts.length} echoes. min {stats[0]}, p50 {stats[1]}, p95 {stats[2]}, max {stats[3]} ms. Fitted offset {offset.toFixed(0)} ms.
      </div>
      <svg viewBox={`0 0 ${frame.width} ${frame.height}`} role="img" aria-label={`RTT histogram, ${run.label}`}>
        <YAxis scale={y} frame={frame} label="Echoes" />
        <XAxis scale={x} frame={frame} label="RTT (ms; the last bar holds the outliers)" />
        {counts.map((c, i) => (
          <rect
            key={i}
            x={x(bins.start + i * bins.width) + 1}
            y={y(c)}
            width={barW}
            height={Math.max(0, y(0) - y(c))}
            rx={2}
            fill={SERIES[0]}
            opacity={hover === null || hover === i ? 1 : 0.6}
          />
        ))}
        <path d={curve} fill="none" stroke={SERIES[1]} strokeWidth={2} pointerEvents="none" />
        <BinTargets count={counts.length} left={(i) => x(bins.start + i * bins.width)} width={barW + 2} frame={frame} onHover={setHover} />
        {hover !== null && (
          <Tooltip
            x={x(bins.start + (hover + 0.5) * bins.width)}
            y={y(counts[hover])}
            frame={frame}
            lines={[`${bins.start + hover * bins.width}-${bins.start + (hover + 1) * bins.width} ms`, `${counts[hover]} echoes (model ${expected[hover].toFixed(1)})`]}
          />
        )}
      </svg>
    </div>
  );
}
