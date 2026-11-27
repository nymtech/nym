"use client";

// Offline model of the client send loop: what your app sends, and what the
// entry gateway sees, for a few traffic presets with Poisson pacing and the
// loop cover stream on or off. No tunnel needed. The maths is in
// lib/speed-test/model.ts; this file only draws it.

import React, { useMemo, useState } from 'react';
import { scaleLinear } from 'd3-scale';
import {
  expectedGapMeanMs,
  histogram,
  simulateClient,
  type PacketKind,
} from '../../lib/speed-test/model';
import { box, legend, row, sub, Button } from './ui';
import { BinTargets, Legend, SERIES, Tooltip, VIZ_CLASS, VizStyle, XAxis, YAxis, innerHeight, svgPoint, type Frame } from './chart';

const WINDOW_MS = 10_000;
const ROW_MS = 2_000;
const ROWS = WINDOW_MS / ROW_MS;

type Preset = 'idle' | 'chat' | 'burst' | 'stream';

const PRESETS: Record<Preset, { label: string; detail: string; sends: () => number[] }> = {
  idle: { label: 'Idle', detail: 'The app sends nothing.', sends: () => [] },
  chat: {
    label: 'Chat',
    detail: 'One single-packet message every 2 s.',
    sends: () => Array.from({ length: 5 }, (_, i) => 500 + i * 2000),
  },
  burst: { label: 'Burst', detail: '40 packets at t = 3 s, for example a page load.', sends: () => new Array<number>(40).fill(3000) },
  stream: {
    label: 'Stream',
    detail: '80 packets per second for the whole window. This is over the 50 per second budget.',
    sends: () => Array.from({ length: (80 * WINDOW_MS) / 1000 }, (_, i) => i * 12.5),
  },
};

const KIND_COLOUR: Record<PacketKind, string> = { real: SERIES[0], cover: SERIES[1], loop: SERIES[2] };
const KIND_LABEL: Record<PacketKind, string> = { real: 'Real packet', cover: 'Main-stream cover', loop: 'Loop cover' };

export function TrafficShapeSimulator() {
  const [preset, setPreset] = useState<Preset>('burst');
  const [poisson, setPoisson] = useState(true);
  const [loopCover, setLoopCover] = useState(true);
  const [observer, setObserver] = useState(false);
  const [seed, setSeed] = useState(1);

  const appSends = useMemo(() => PRESETS[preset].sends(), [preset]);
  // Seeded and deterministic, so the same inputs always give the same picture.
  const sim = useMemo(
    () => simulateClient({ appSends, poisson, loopCover, durationMs: WINDOW_MS, seed }),
    [appSends, poisson, loopCover, seed],
  );

  const real = sim.packets.filter((p) => p.kind === 'real').length;
  const perSecond = sim.packets.length / (WINDOW_MS / 1000);

  return (
    <div className={VIZ_CLASS} style={box}>
      <VizStyle />
      <div style={legend}>Traffic-shape simulator</div>
      <div style={row}>
        {(Object.keys(PRESETS) as Preset[]).map((p) => (
          <label key={p} style={{ ...sub, opacity: 1, display: 'inline-flex', gap: 4, alignItems: 'center' }}>
            <input type="radio" name="sim-preset" checked={preset === p} onChange={() => setPreset(p)} />
            {PRESETS[p].label}
          </label>
        ))}
      </div>
      <div style={sub}>{PRESETS[preset].detail}</div>
      <div style={{ ...row, marginTop: '0.5rem' }}>
        <label style={{ ...sub, opacity: 1 }}>
          <input type="checkbox" checked={poisson} onChange={(e) => setPoisson(e.target.checked)} /> Poisson pacing
        </label>
        <label style={{ ...sub, opacity: 1 }}>
          <input type="checkbox" checked={loopCover} onChange={(e) => setLoopCover(e.target.checked)} /> Loop cover stream
        </label>
        <label style={{ ...sub, opacity: 1 }}>
          <input type="checkbox" checked={observer} onChange={(e) => setObserver(e.target.checked)} /> Observer view (one colour)
        </label>
        <Button onClick={() => setSeed(Math.floor(Math.random() * 1e9))}>Reshuffle</Button>
      </div>

      {observer ? (
        <Legend items={[{ label: 'Packet (an observer cannot tell real from cover)', colour: 'var(--viz-neutral)' }]} />
      ) : (
        <Legend items={(['real', 'cover', 'loop'] as PacketKind[]).map((k) => ({ label: KIND_LABEL[k], colour: KIND_COLOUR[k] }))} />
      )}
      <Timeline appSends={appSends} packets={sim.packets} observer={observer} />

      <table style={{ fontSize: 12, margin: '0.75rem 0', borderCollapse: 'collapse' }}>
        <tbody>
          <StatRow label="Packets at the gateway" value={`${perSecond.toFixed(1)} per second`} />
          <StatRow label="Real packets sent" value={`${real} of ${appSends.length} (${sim.packets.length ? ((100 * real) / sim.packets.length).toFixed(0) : 0}% of all packets)`} />
          <StatRow label="Mean wait in the send queue" value={sim.meanWaitMs === null ? 'none sent' : `${sim.meanWaitMs.toFixed(0)} ms`} />
          <StatRow label="Still queued at 10 s" value={String(sim.unsent)} />
        </tbody>
      </table>

      {poisson ? (
        <>
          <QueueChart queue={sim.queue} />
          <GapHistogram packets={sim.packets} loopCover={loopCover} />
        </>
      ) : (
        <div style={sub}>With Poisson pacing off, packets leave when the app sends them. There is no queue and no main-stream cover, so the gateway sees the shape of your traffic.</div>
      )}
      <div style={{ ...sub, marginTop: '0.5rem' }}>
        This is a model of the client send loop. It does not include gateway backpressure, which can slow the client, or retransmissions of lost packets.
      </div>
    </div>
  );
}

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <tr>
      <td style={{ padding: '2px 12px 2px 0', opacity: 0.7 }}>{label}</td>
      <td style={{ padding: '2px 0', fontVariantNumeric: 'tabular-nums' }}>{value}</td>
    </tr>
  );
}

// Five rows of 2 s each: at ~55 packets per second a single 10 s row would be a
// solid bar. Each row has an app lane above a gateway lane.
function Timeline({ appSends, packets, observer }: { appSends: number[]; packets: { t: number; kind: PacketKind }[]; observer: boolean }) {
  const width = 720;
  const left = 70;
  const rowH = 44;
  const x = scaleLinear().domain([0, ROW_MS]).range([left, width - 8]);
  const counts = new Map<number, number>();
  for (const t of appSends) counts.set(t, (counts.get(t) ?? 0) + 1);

  return (
    <svg viewBox={`0 0 ${width} ${ROWS * rowH + 6}`} role="img" aria-label="Timeline of app sends and packets at the gateway">
      {Array.from({ length: ROWS }, (_, r) => {
        const y0 = r * rowH + 4;
        const from = r * ROW_MS;
        const inRow = (t: number) => t >= from && t < from + ROW_MS;
        return (
          <g key={r}>
            <text x={0} y={y0 + 9}>App</text>
            <text x={0} y={y0 + 27}>Gateway</text>
            <text x={width - 8} y={y0 + 40} textAnchor="end">{`${from / 1000}-${(from + ROW_MS) / 1000} s`}</text>
            <line x1={left} x2={width - 8} y1={y0 + 33} y2={y0 + 33} stroke="var(--viz-grid)" />
            {[...counts].filter(([t]) => inRow(t)).map(([t, n]) => (
              <g key={t}>
                <rect x={x(t - from) - 1} y={y0 + 2} width={2} height={10} rx={1} fill={observer ? 'var(--viz-neutral)' : KIND_COLOUR.real} />
                {n > 1 && <text x={x(t - from) + 4} y={y0 + 11}>{`×${n}`}</text>}
              </g>
            ))}
            {packets.filter((p) => inRow(p.t)).map((p, i) => (
              <rect
                key={i}
                x={x(p.t - from) - 1}
                y={y0 + 16}
                width={2}
                height={16}
                rx={1}
                fill={observer ? 'var(--viz-neutral)' : KIND_COLOUR[p.kind]}
              />
            ))}
          </g>
        );
      })}
    </svg>
  );
}

function QueueChart({ queue }: { queue: { t: number; depth: number }[] }) {
  const frame: Frame = { width: 720, height: 150, left: 44, right: 8, top: 22, bottom: 30 };
  const [hover, setHover] = useState<{ t: number; depth: number } | null>(null);
  const max = Math.max(1, ...queue.map((q) => q.depth));
  const x = scaleLinear().domain([0, WINDOW_MS]).range([frame.left, frame.width - frame.right]);
  const y = scaleLinear().domain([0, max]).nice().range([frame.height - frame.bottom, frame.top]);
  const path = queue.map((q, i) => `${i ? 'L' : 'M'}${x(q.t).toFixed(1)},${y(q.depth).toFixed(1)}`).join('');

  function onMove(e: React.PointerEvent<SVGRectElement>) {
    const t = x.invert(svgPoint(e, frame).x);
    let best = queue[0];
    for (const q of queue) if (Math.abs(q.t - t) < Math.abs(best.t - t)) best = q;
    setHover(best ?? null);
  }

  return (
    <svg viewBox={`0 0 ${frame.width} ${frame.height}`} role="img" aria-label="Send queue depth over time">
      <YAxis scale={y} frame={frame} label="Packets waiting in the send queue" />
      <XAxis scale={x} frame={frame} label="Time (s)" format={(v) => String(v / 1000)} />
      <path d={path} fill="none" stroke={SERIES[0]} strokeWidth={2} />
      {hover && (
        <>
          <line x1={x(hover.t)} x2={x(hover.t)} y1={frame.top} y2={frame.height - frame.bottom} stroke="var(--viz-axis)" />
          <circle cx={x(hover.t)} cy={y(hover.depth)} r={4} fill={SERIES[0]} stroke="var(--viz-surface)" strokeWidth={2} />
          <Tooltip x={x(hover.t)} y={y(hover.depth)} frame={frame} lines={[`${(hover.t / 1000).toFixed(2)} s`, `${hover.depth} queued`]} />
        </>
      )}
      <rect
        x={frame.left}
        y={frame.top}
        width={frame.width - frame.left - frame.right}
        height={innerHeight(frame)}
        fill="transparent"
        onPointerMove={onMove}
        onPointerLeave={() => setHover(null)}
      />
    </svg>
  );
}

// Gaps between packets at the gateway, with the exponential curve the merged
// Poisson streams should follow.
function GapHistogram({ packets, loopCover }: { packets: { t: number }[]; loopCover: boolean }) {
  const frame: Frame = { width: 720, height: 170, left: 44, right: 8, top: 22, bottom: 30 };
  const bins = { start: 0, width: 5, count: 20 };
  const gaps = packets.slice(1).map((p, i) => p.t - packets[i].t);
  const counts = histogram(gaps, bins);
  const mean = expectedGapMeanMs(true, loopCover) as number;
  const expected = counts.map((_, i) => {
    const a = bins.start + i * bins.width;
    // The last bin holds the whole tail, to match histogram()'s clamping.
    const upper = i === bins.count - 1 ? 0 : Math.exp(-(a + bins.width) / mean);
    return gaps.length * (Math.exp(-a / mean) - upper);
  });
  const [hover, setHover] = useState<number | null>(null);

  const x = scaleLinear().domain([0, bins.width * bins.count]).range([frame.left, frame.width - frame.right]);
  const y = scaleLinear().domain([0, Math.max(1, ...counts, ...expected)]).nice().range([frame.height - frame.bottom, frame.top]);
  const curve = expected.map((v, i) => `${i ? 'L' : 'M'}${x(i * bins.width + bins.width / 2).toFixed(1)},${y(v).toFixed(1)}`).join('');
  const barW = x(bins.width) - x(0) - 2;

  return (
    <>
      <Legend items={[{ label: 'Gaps in this run', colour: SERIES[0] }, { label: `Exponential, mean ${mean.toFixed(1)} ms`, colour: SERIES[1], shape: 'line' }]} />
      <svg viewBox={`0 0 ${frame.width} ${frame.height}`} role="img" aria-label="Histogram of gaps between packets at the gateway">
        <YAxis scale={y} frame={frame} label="Number of gaps" />
        <XAxis scale={x} frame={frame} label="Gap between packets (ms; the last bar holds all longer gaps)" />
        {counts.map((c, i) => (
          <rect
            key={i}
            x={x(i * bins.width) + 1}
            y={y(c)}
            width={barW}
            height={Math.max(0, y(0) - y(c))}
            rx={2}
            fill={SERIES[0]}
            opacity={hover === null || hover === i ? 1 : 0.6}
          />
        ))}
        <path d={curve} fill="none" stroke={SERIES[1]} strokeWidth={2} pointerEvents="none" />
        <BinTargets count={counts.length} left={(i) => x(i * bins.width)} width={barW + 2} frame={frame} onHover={setHover} />
        {hover !== null && (
          <Tooltip
            x={x(hover * bins.width + bins.width / 2)}
            y={y(counts[hover])}
            frame={frame}
            lines={[`${hover * bins.width}-${(hover + 1) * bins.width} ms`, `${counts[hover]} gaps (model ${expected[hover].toFixed(0)})`]}
          />
        )}
      </svg>
    </>
  );
}
