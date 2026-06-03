// Shared scaffolding for the mix-* live demos (mix-fetch, mix-dns,
// mix-websocket, mix-tunnel).
//
// WHY THE @ts-ignore'd DYNAMIC IMPORT (in each demo, not here): these
// components are committed before the `@nymproject/mix-*` v2 packages are
// published to npm. The docs site type-checks every .tsx (tsconfig
// `include: **/*.tsx`) and `next build` fails on type errors, so a normal
// `import { mixFetch } from '@nymproject/mix-fetch'` would break the build with
// "cannot find module" the moment it lands. Each demo instead does:
//
//   // @ts-ignore -- published separately; absent at build time pre-publish
//   const mod = (await import('@nymproject/mix-fetch')) as unknown as MixFetchModule;
//
// The `@ts-ignore` suppresses the missing-module error pre-publish (and is a
// harmless no-op once the package exists). The specifier stays a LITERAL so
// that post-publish webpack code-splits the package into an async chunk and
// lazy-loads it — the multi-MB wasm only loads when a handler runs, never on
// page render. We cast through our local interfaces (below) to keep call sites
// typed despite the `any`.
//
// TO ACTIVATE A DEMO once the packages are on npm: add, to the relevant page,
//   import { MixFetchDemo } from '../../components/mix-demos/mix-fetch-demo'
// and drop `<MixFetchDemo />` where the live block should appear. No change to
// these component files is needed.

import React, { useCallback, useRef, useState } from 'react';

// Local mirror of the bits of `@nymproject/mix-tunnel`'s SetupMixTunnelOpts we
// surface in the demos. The published type is the source of truth; this is a
// deliberately small subset for the demo UI.
export interface DemoSetupOpts {
  preferredIpr?: string;
  disableCoverTraffic?: boolean;
  disablePoissonTraffic?: boolean;
  primaryDns?: string;
  debug?: boolean;
}

// Minimal shapes of each package's runtime surface, used to cast the `any`
// returned by the dynamic import back to something typed.
export interface MixTunnelModule {
  setupMixTunnel(opts?: DemoSetupOpts): Promise<void>;
  disconnectMixTunnel(): Promise<void>;
  getTunnelState(): Promise<{ state: string; reason?: string }>;
}
export interface MixFetchModule extends MixTunnelModule {
  mixFetch(url: string, init?: RequestInit): Promise<Response>;
}
export interface MixDnsModule extends MixTunnelModule {
  mixDNS(hostname: string): Promise<string>;
}
export interface MixWebSocketModule extends MixTunnelModule {
  MixWebSocket: new (url: string, protocols?: string | string[]) => MixWebSocketLike;
}
export interface MixWebSocketLike extends EventTarget {
  send(data: string | ArrayBufferView | ArrayBuffer): Promise<void>;
  close(code?: number, reason?: string): Promise<void>;
  opened(): Promise<void>;
  readyState: number;
}

// A small append-only log panel, the React analogue of the `<pre id="output">`
// the standalone example apps write to.
export function useLog() {
  const [lines, setLines] = useState<string[]>([]);
  const log = useCallback((line: string) => setLines((prev) => [...prev, line]), []);
  const clear = useCallback(() => setLines([]), []);
  return { lines, log, clear };
}

export function LogPanel({ lines }: { lines: string[] }) {
  const ref = useRef<HTMLPreElement>(null);
  React.useEffect(() => {
    if (ref.current) ref.current.scrollTop = ref.current.scrollHeight;
  }, [lines]);
  return (
    <pre
      ref={ref}
      style={{
        maxHeight: 240,
        overflowY: 'auto',
        padding: '0.75rem',
        borderRadius: 6,
        background: 'rgba(127,127,127,0.08)',
        border: '1px solid rgba(127,127,127,0.3)',
        fontSize: 13,
        lineHeight: 1.5,
        margin: 0,
      }}
    >
      {lines.length ? lines.join('\n') : 'Idle. Press Run to bring up the tunnel.'}
    </pre>
  );
}

// Wrapper giving every demo the same title bar and the log panel underneath the
// controls.
export function DemoFrame({
  title,
  children,
  lines,
}: {
  title: string;
  children: React.ReactNode;
  lines: string[];
}) {
  return (
    <div
      style={{
        border: '1px solid rgba(127,127,127,0.3)',
        borderRadius: 8,
        padding: '1rem',
        margin: '1.5rem 0',
      }}
    >
      <div
        style={{
          display: 'flex',
          alignItems: 'baseline',
          justifyContent: 'space-between',
          marginBottom: '0.75rem',
        }}
      >
        <strong>{title}</strong>
        <span style={{ fontSize: 12, opacity: 0.6 }}>runs in your browser, over the mixnet</span>
      </div>
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: '0.5rem', marginBottom: '0.75rem' }}>
        {children}
      </div>
      <LogPanel lines={lines} />
    </div>
  );
}

// Shared control styles so the demos stay visually consistent without pulling
// in a component library.
export const btnStyle: React.CSSProperties = {
  padding: '0.4rem 0.9rem',
  borderRadius: 6,
  border: '1px solid rgba(127,127,127,0.4)',
  background: 'transparent',
  cursor: 'pointer',
  fontSize: 14,
};
export const inputStyle: React.CSSProperties = {
  padding: '0.4rem 0.6rem',
  borderRadius: 6,
  border: '1px solid rgba(127,127,127,0.4)',
  background: 'transparent',
  fontSize: 14,
  flex: '1 1 16rem',
};
