// Shared presentational primitives for the playground sections (MixPlayground
// and the raw-messaging demo) so they share one look. Theme-neutral inline
// styles (rgba greys read on light and dark Nextra themes), a per-section log
// store, and an autoscrolling log panel.

import React, { useCallback, useEffect, useRef, useState } from 'react';

export type Colour = 'green' | 'red' | 'orange' | 'gray' | undefined;
export const COLOURS: Record<string, string> = {
  green: '#16a34a',
  red: '#dc2626',
  orange: '#d97706',
  gray: '#9ca3af',
};

export interface LogEntry {
  ts: string;
  msg: string;
  colour?: Colour;
}

// One append-only buffer per section, keyed by section name. Red entries mirror
// to console.error so they sit alongside the Rust-side `[smolmix] ...` logs.
export function useLogs() {
  const [store, setStore] = useState<Record<string, LogEntry[]>>({});
  const log = useCallback((section: string, msg: string, colour?: Colour) => {
    const ts = new Date().toISOString().slice(11, 23);
    if (colour === 'red') console.error(`[smolmix-playground:${section}]`, msg);
    setStore((s) => ({ ...s, [section]: [...(s[section] ?? []), { ts, msg, colour }] }));
  }, []);
  const lines = useCallback((section: string) => store[section] ?? [], [store]);
  return { log, lines };
}

export const box: React.CSSProperties = {
  border: '1px solid rgba(127,127,127,0.3)',
  borderRadius: 8,
  padding: '1rem',
  margin: '1rem 0',
};
export const row: React.CSSProperties = {
  display: 'flex',
  gap: '0.5rem',
  alignItems: 'center',
  flexWrap: 'wrap',
  marginBottom: '0.5rem',
};
export const btn: React.CSSProperties = {
  padding: '0.35rem 0.8rem',
  borderRadius: 6,
  border: '1px solid rgba(127,127,127,0.4)',
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 14,
};
export const input: React.CSSProperties = {
  padding: '0.35rem 0.6rem',
  borderRadius: 6,
  border: '1px solid rgba(127,127,127,0.4)',
  background: 'transparent',
  fontSize: 14,
  flex: '1 1 14rem',
  minWidth: 0,
};
export const num: React.CSSProperties = { ...input, flex: '0 0 5rem' };
export const legend: React.CSSProperties = { fontWeight: 600, marginBottom: '0.6rem' };
export const sub: React.CSSProperties = { fontSize: 12, opacity: 0.65 };

export function Button(props: React.ButtonHTMLAttributes<HTMLButtonElement>) {
  return (
    <button
      {...props}
      style={{ ...btn, ...(props.disabled ? { opacity: 0.4, cursor: 'not-allowed' } : {}) }}
    />
  );
}

export function LogPanel({ lines, placeholder }: { lines: LogEntry[]; placeholder?: string }) {
  const ref = useRef<HTMLPreElement>(null);
  useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines]);
  return (
    <pre
      ref={ref}
      style={{
        maxHeight: 220,
        overflowY: 'auto',
        padding: '0.6rem',
        borderRadius: 6,
        background: 'rgba(127,127,127,0.08)',
        border: '1px solid rgba(127,127,127,0.25)',
        fontSize: 12.5,
        lineHeight: 1.5,
        margin: '0.5rem 0 0',
        whiteSpace: 'pre-wrap',
      }}
    >
      {lines.length === 0
        ? placeholder ?? 'Idle.'
        : lines.map((l, i) => (
            <div key={i} style={l.colour ? { color: COLOURS[l.colour] } : undefined}>
              [{l.ts}] {l.msg}
            </div>
          ))}
    </pre>
  );
}

export interface Status {
  text: string;
  colour?: Colour;
}

export function StatusText({ status }: { status: Status }) {
  return (
    <span style={{ ...sub, color: status.colour ? COLOURS[status.colour] : undefined }}>
      {status.text}
    </span>
  );
}

// A small CSS spinner with optional label. Used while a tunnel request is in
// flight, since mixFetch buffers the whole body and exposes no byte progress;
// the live transport detail goes to the browser console instead.
export function Spinner({ label }: { label?: string }) {
  return (
    <span style={{ ...sub, display: 'inline-flex', alignItems: 'center', gap: 6 }}>
      <style>{'@keyframes mixspin{to{transform:rotate(360deg)}}'}</style>
      <span
        aria-hidden
        style={{
          width: 11,
          height: 11,
          border: '2px solid rgba(127,127,127,0.35)',
          borderTopColor: COLOURS.orange,
          borderRadius: '50%',
          display: 'inline-block',
          animation: 'mixspin 0.7s linear infinite',
        }}
      />
      {label}
    </span>
  );
}
