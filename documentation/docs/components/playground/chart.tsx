"use client";

// Small SVG chart pieces for the speed-test simulator and live tests: theme
// tokens, axes, legend and a hover tooltip. Charts use d3-scale for the maths
// and plain SVG for the marks, so no chart library is needed.

import React from 'react';
import type { ScaleLinear } from 'd3-scale';

export const VIZ_CLASS = 'nym-speed-viz';

// The docs toggle themes with `html.dark`, so light is the base and dark
// overrides under html.dark. Series hues are the first three slots of the
// validated categorical palette (they pass all-pairs CVD checks in both modes).
const CSS = `
.${VIZ_CLASS} {
  --viz-surface: #fcfcfb;
  --viz-text: #0b0b0b;
  --viz-text-2: #52514e;
  --viz-grid: rgba(127,127,127,0.18);
  --viz-axis: rgba(127,127,127,0.55);
  --viz-s1: #2a78d6;
  --viz-s2: #eb6834;
  --viz-s3: #1baf7a;
  --viz-neutral: #8a8984;
}
html.dark .${VIZ_CLASS} {
  --viz-surface: #1a1a19;
  --viz-text: #f0efec;
  --viz-text-2: #c3c2b7;
  --viz-s1: #3987e5;
  --viz-s2: #d95926;
  --viz-s3: #199e70;
  --viz-neutral: #8f8e88;
}
.${VIZ_CLASS} svg { display: block; width: 100%; height: auto; overflow: visible; }
.${VIZ_CLASS} text { fill: var(--viz-text-2); font-size: 11px; }
`;

export const SERIES = ['var(--viz-s1)', 'var(--viz-s2)', 'var(--viz-s3)'] as const;

export function VizStyle() {
  return <style>{CSS}</style>;
}

export interface Frame {
  width: number;
  height: number;
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export const innerWidth = (f: Frame) => f.width - f.left - f.right;
export const innerHeight = (f: Frame) => f.height - f.top - f.bottom;

export function XAxis({ scale, frame, label, ticks = 6, format = String }: {
  scale: ScaleLinear<number, number>;
  frame: Frame;
  label: string;
  ticks?: number;
  format?: (v: number) => string;
}) {
  const y = frame.height - frame.bottom;
  return (
    <g>
      <line x1={frame.left} x2={frame.width - frame.right} y1={y} y2={y} stroke="var(--viz-axis)" />
      {scale.ticks(ticks).map((t) => (
        <text key={t} x={scale(t)} y={y + 14} textAnchor="middle">{format(t)}</text>
      ))}
      <text x={frame.left + innerWidth(frame) / 2} y={frame.height - 2} textAnchor="middle">{label}</text>
    </g>
  );
}

export function YAxis({ scale, frame, label, ticks = 4, format = String }: {
  scale: ScaleLinear<number, number>;
  frame: Frame;
  label: string;
  ticks?: number;
  format?: (v: number) => string;
}) {
  return (
    <g>
      {scale.ticks(ticks).map((t) => (
        <g key={t}>
          <line x1={frame.left} x2={frame.width - frame.right} y1={scale(t)} y2={scale(t)} stroke="var(--viz-grid)" />
          <text x={frame.left - 6} y={scale(t) + 3} textAnchor="end">{format(t)}</text>
        </g>
      ))}
      <text x={frame.left} y={frame.top - 8} textAnchor="start">{label}</text>
    </g>
  );
}

export function Legend({ items }: { items: { label: string; colour: string; shape?: 'dot' | 'line' }[] }) {
  return (
    <div style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', fontSize: 12, margin: '0.25rem 0' }}>
      {items.map((it) => (
        <span key={it.label} style={{ display: 'inline-flex', alignItems: 'center', gap: 6 }}>
          <span
            aria-hidden
            style={
              it.shape === 'line'
                ? { width: 16, height: 2, background: it.colour, display: 'inline-block' }
                : { width: 9, height: 9, borderRadius: '50%', background: it.colour, display: 'inline-block' }
            }
          />
          {it.label}
        </span>
      ))}
    </div>
  );
}

/** Tooltip text at an SVG position, drawn as an SVG label with a backing box. */
export function Tooltip({ x, y, lines, frame }: { x: number; y: number; lines: string[]; frame: Frame }) {
  const w = 8 + 6.4 * Math.max(...lines.map((l) => l.length));
  const h = 6 + 14 * lines.length;
  const left = Math.min(Math.max(x + 10, frame.left), frame.width - w);
  const top = Math.max(y - h - 8, 0);
  return (
    <g pointerEvents="none">
      <rect x={left} y={top} width={w} height={h} rx={4} fill="var(--viz-surface)" stroke="var(--viz-axis)" />
      {lines.map((l, i) => (
        <text key={i} x={left + 4} y={top + 15 + i * 14} style={{ fill: 'var(--viz-text)' }}>{l}</text>
      ))}
    </g>
  );
}

/** Convert a pointer event to SVG viewBox coordinates. */
export function svgPoint(e: React.PointerEvent<SVGElement>, frame: Frame): { x: number; y: number } {
  const svg = (e.currentTarget as SVGElement).ownerSVGElement ?? (e.currentTarget as SVGSVGElement);
  const r = svg.getBoundingClientRect();
  return { x: ((e.clientX - r.left) / r.width) * frame.width, y: ((e.clientY - r.top) / r.height) * frame.height };
}

/** A transparent full-height hover target for each bin, so empty bins can be inspected too. */
export function BinTargets({ count, left, width, frame, onHover }: {
  count: number;
  /** SVG x of the left edge of bin i. */
  left: (i: number) => number;
  width: number;
  frame: Frame;
  onHover: (i: number | null) => void;
}) {
  return (
    <g fill="transparent" onPointerLeave={() => onHover(null)}>
      {Array.from({ length: count }, (_, i) => (
        <rect key={i} x={left(i)} y={frame.top} width={width} height={innerHeight(frame)} onPointerEnter={() => onHover(i)} />
      ))}
    </g>
  );
}
