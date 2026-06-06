// Pure helpers + package loader for the mix playground. No React here: this is
// the logic ported from `wasm/smolmix/internal-dev/index.js`, adapted to drive
// the *published* @nymproject/mix-* packages instead of internal-dev's own
// worker. The differences that matter:
//   - published `mixFetch` returns a real `Response` (internal-dev returned a
//     raw `{body,status,statusText,headers}` and wrapped it);
//   - there is no live `setDebugLogging`; debug is a `setupMixTunnel` opt;
//   - the WebSocket is the EventTarget-based `MixWebSocket` (async send/close,
//     `opened()`, binaryType fixed to arraybuffer).

// Local mirror of the published `SetupMixTunnelOpts` (subset we surface).
export interface SetupOpts {
  preferredIpr?: string;
  clientId?: string;
  forceTls?: boolean;
  disablePoissonTraffic?: boolean;
  disableCoverTraffic?: boolean;
  openReplySurbs?: number;
  dataReplySurbs?: number;
  primaryDns?: string;
  fallbackDns?: string;
  debug?: boolean;
}

export interface TunnelState {
  state: string;
  reason?: string;
}

// Mirror of the published `MixWebSocket` runtime surface.
export interface MixWebSocketLike extends EventTarget {
  send(data: string | ArrayBuffer | Uint8Array): Promise<void>;
  close(code?: number, reason?: string): Promise<void>;
  opened(): Promise<void>;
  readonly readyState: number;
  readonly protocols: string[];
}

// The slice of the three packages the playground uses. They share one tunnel
// (mix-tunnel is deduped), so setup/disconnect/state are taken from mix-fetch.
export interface PlaygroundMods {
  setupMixTunnel(opts?: SetupOpts): Promise<void>;
  disconnectMixTunnel(): Promise<void>;
  getTunnelState(): Promise<TunnelState>;
  mixFetch(url: string, init?: RequestInit): Promise<Response>;
  mixDNS(hostname: string): Promise<string>;
  MixWebSocket: new (url: string, protocols?: string | string[]) => MixWebSocketLike;
}

// Lazy-load the three facades. Literal specifiers keep webpack code-splitting
// the wasm into async chunks (loaded only when the user clicks Setup). The
// `@ts-ignore`s are harmless once installed; they keep a fresh checkout
// type-checking before `pnpm install`.
export async function loadModules(): Promise<PlaygroundMods> {
  const [f, d, w] = await Promise.all([
    // @ts-ignore -- @nymproject/mix-fetch resolves at runtime; lazy wasm chunk
    import('@nymproject/mix-fetch'),
    // @ts-ignore -- @nymproject/mix-dns resolves at runtime; lazy wasm chunk
    import('@nymproject/mix-dns'),
    // @ts-ignore -- @nymproject/mix-websocket resolves at runtime; lazy wasm chunk
    import('@nymproject/mix-websocket'),
  ]);
  return {
    setupMixTunnel: f.setupMixTunnel,
    disconnectMixTunnel: f.disconnectMixTunnel,
    getTunnelState: f.getTunnelState,
    mixFetch: f.mixFetch,
    mixDNS: d.mixDNS,
    MixWebSocket: w.MixWebSocket,
  } as unknown as PlaygroundMods;
}

// Fresh client-storage id per page load so a reload gets a clean identity and
// doesn't collide with the gateway connection from the previous load (which
// lingers in the gateway's post-disconnect grace window). See tunnel.rs:
// "Randomise per session to get a clean client".
//
// Called from a post-mount effect, never at module/render time: Math.random at
// render would differ between SSG and the client and trip React hydration.
export const randomClientId = () => `smolmix-playground-${Math.random().toString(36).slice(2, 8)}`;

export const clampSurbs = (n: number) => Math.min(50, Math.max(0, n));

export function formatSize(bytes: number): string {
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + ' MB';
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return bytes + ' B';
}

export function formatRate(bytes: number, ms: number): string {
  return (bytes / 1024 / (ms / 1000)).toFixed(1) + ' KB/s';
}

export function hexPreview(data: Uint8Array | ArrayBuffer, maxBytes = 64): string {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  const len = Math.min(bytes.length, maxBytes);
  const hex = Array.from(bytes.slice(0, len), (b) => b.toString(16).padStart(2, '0')).join(' ');
  return bytes.length > maxBytes ? `${hex} ...` : hex;
}

export async function sha256hex(buf: ArrayBuffer): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', buf);
  return Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, '0')).join('');
}

export function saveFile(buf: ArrayBuffer, filename: string, mimeType: string): void {
  const blob = new Blob([buf], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// The browser exposes no raw DNS API; "clearnet DNS" from JS is DoH (HTTPS) to
// a public resolver. Google's JSON API is CORS-friendly and returns
// { Status, Answer: [{ name, type, TTL, data }] } where type=1 is an A record.
// The request shows up in DevTools Network as a plain HTTPS fetch.
export async function dohResolve(hostname: string): Promise<string> {
  const resp = await window.fetch(
    `https://dns.google/resolve?name=${encodeURIComponent(hostname)}&type=A`,
    { mode: 'cors' },
  );
  const json = await resp.json();
  if (json.Status !== 0) throw new Error(`DoH status=${json.Status}`);
  const a = (json.Answer as Array<{ type: number; data: string }> | undefined)?.find(
    (x) => x.type === 1,
  );
  if (!a) throw new Error('no A record');
  return a.data;
}

// Stress-test request generation (uniform / mixed / drip), ported verbatim.
export interface StressRequest {
  id: number;
  url: string;
  label: string;
}

const SIZE_PROFILES = [
  { label: 'tiny', bytes: 128 },
  { label: 'small', bytes: 1024 },
  { label: 'medium', bytes: 10240 },
  { label: 'large', bytes: 102400 },
  { label: 'xlarge', bytes: 1048576 },
];

function buildDripProfiles(timeoutSec: number) {
  return [
    { label: 'safe', duration: Math.round(timeoutSec * 0.5), delay: 0, bytes: 100 },
    { label: 'boundary', duration: Math.round(timeoutSec * 0.92), delay: 0, bytes: 100 },
    { label: 'over', duration: Math.round(timeoutSec * 1.08), delay: 0, bytes: 100 },
    {
      label: 'slow-start',
      duration: Math.round(timeoutSec * 0.83),
      delay: Math.round(timeoutSec * 0.17),
      bytes: 100,
    },
  ];
}

export function generateRequests(
  count: number,
  mode: 'uniform' | 'mixed' | 'drip',
  timeoutSec: number,
  baseUrl: string,
): StressRequest[] {
  const requests: StressRequest[] = [];
  if (mode === 'uniform') {
    for (let i = 1; i <= count; i++) requests.push({ id: i, url: `${baseUrl}${i}`, label: 'uniform' });
  } else if (mode === 'mixed') {
    for (let i = 1; i <= count; i++) {
      const p = SIZE_PROFILES[Math.floor(Math.random() * SIZE_PROFILES.length)];
      requests.push({ id: i, url: `https://httpbin.org/bytes/${p.bytes}`, label: p.label });
    }
  } else {
    const profiles = buildDripProfiles(timeoutSec);
    for (let i = 1; i <= count; i++) {
      const p = profiles[Math.floor(Math.random() * profiles.length)];
      requests.push({
        id: i,
        url: `https://httpbin.org/drip?duration=${p.duration}&numbytes=${p.bytes}&delay=${p.delay}&code=200`,
        label: p.label,
      });
    }
  }
  return requests;
}
