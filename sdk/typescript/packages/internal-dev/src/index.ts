// smolmix TS SDK dev — mirrors wasm/smolmix/internal-dev so the two
// playgrounds run the same scenarios; the SDK layer adds (mostly) nothing
// the raw WASM doesn't already do, so observed behaviour should match.

import {
  setupMixTunnel,
  disconnectMixTunnel,
  SetupMixTunnelOpts,
} from '@nymproject/mix-tunnel';
import { mixFetch } from '@nymproject/mix-fetch';
import { mixDNS } from '@nymproject/mix-dns';
import { MixWebSocket } from '@nymproject/mix-websocket';

// Helpers ============================================================

const $ = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} not found`);
  return el as T;
};

type LogColour = 'green' | 'red' | 'orange' | 'gray' | undefined;

function display(msg: string, colour?: LogColour) {
  const ts = new Date().toISOString().slice(11, 23);
  const line = document.createElement('div');
  if (colour) line.style.color = colour;
  line.textContent = `[${ts}] ${msg}`;
  const out = $('output');
  out.appendChild(line);
  out.scrollTop = out.scrollHeight;
  if (colour === 'red') console.error('[sdk-dev]', msg);
}

function logTo(targetId: string, msg: string, colour?: LogColour) {
  const target = document.getElementById(targetId);
  if (!target) return;
  const ts = new Date().toISOString().slice(11, 23);
  const line = document.createElement('div');
  if (colour) line.style.color = colour;
  line.textContent = `[${ts}] ${msg}`;
  target.appendChild(line);
  target.scrollTop = target.scrollHeight;
  if (colour === 'red') console.error(`[sdk-dev:${targetId}]`, msg);
}

const formatSize = (bytes: number): string => {
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
};

const formatRate = (bytes: number, ms: number): string =>
  `${(bytes / 1024 / (ms / 1000)).toFixed(1)} KB/s`;

const hexPreview = (data: Uint8Array | ArrayBuffer, maxBytes = 64): string => {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  const len = Math.min(bytes.length, maxBytes);
  const hex = Array.from(bytes.slice(0, len), (b) => b.toString(16).padStart(2, '0')).join(' ');
  return bytes.length > maxBytes ? `${hex} ...` : hex;
};

async function sha256hex(bytes: BufferSource): Promise<string> {
  const hash = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(hash), (b) => b.toString(16).padStart(2, '0')).join('');
}

function saveFile(buf: BlobPart, filename: string, mimeType: string) {
  const blob = new Blob([buf], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// Tunnel-gated UI bits. GET + DNS fieldsets stay enabled so their clearnet
// buttons work without a tunnel; only the per-button gates are toggled.
const GATED_FIELDSETS = ['ws-controls', 'stress-controls', 'download-controls'];
const GATED_BUTTONS = ['btn-get-tunnel', 'btn-dns-tunnel'];

function setTunnelButtonsEnabled(enabled: boolean) {
  for (const id of GATED_FIELDSETS) ($(id) as HTMLFieldSetElement).disabled = !enabled;
  for (const id of GATED_BUTTONS) ($(id) as HTMLButtonElement).disabled = !enabled;
}

const strField = (id: string): string | undefined => {
  const v = ($(id) as HTMLInputElement).value.trim();
  return v.length === 0 ? undefined : v;
};
const numField = (id: string): number | undefined => {
  const v = ($(id) as HTMLInputElement).value.trim();
  if (!v) return undefined;
  const n = Number(v);
  return Number.isFinite(n) ? n : undefined;
};

// Override the default User-Agent header injected by the wasm-shim. Kept in
// module scope so each fetch can reach it without re-reading the input.
let userAgentOverride: string | undefined;
const fetchInit = (): RequestInit | undefined =>
  userAgentOverride ? { headers: { 'User-Agent': userAgentOverride } } : undefined;

// Connection =========================================================

$('opt-random-ipr').addEventListener('change', (e) => {
  $('ipr-address').toggleAttribute('disabled', (e.target as HTMLInputElement).checked);
});

$('btn-setup').addEventListener('click', async () => {
  const useRandom = ($('opt-random-ipr') as HTMLInputElement).checked;
  const ipr = useRandom ? undefined : ($('ipr-address') as HTMLInputElement).value.trim();
  if (!useRandom && !ipr) {
    display('IPR address is required (or check "Use random IPR")', 'red');
    return;
  }

  const statusEl = $('tunnel-status');
  ($('btn-setup') as HTMLButtonElement).disabled = true;
  statusEl.textContent = 'Connecting...';
  statusEl.style.color = 'orange';

  userAgentOverride = strField('opt-user-agent');

  const opts: SetupMixTunnelOpts = {
    ...(ipr ? { preferredIpr: ipr } : {}),
    clientId: strField('opt-client-id'),
    forceTls: ($('opt-force-tls') as HTMLInputElement).checked,
    disablePoissonTraffic: ($('opt-disable-poisson') as HTMLInputElement).checked,
    disableCoverTraffic: ($('opt-disable-cover') as HTMLInputElement).checked,
    openReplySurbs: numField('opt-open-surbs'),
    dataReplySurbs: numField('opt-data-surbs'),
    primaryDns: strField('opt-primary-dns'),
    fallbackDns: strField('opt-fallback-dns'),
    debug: ($('opt-debug-logging') as HTMLInputElement).checked,
  };

  display(`setupMixTunnel (clientId=${opts.clientId}, IPR: ${ipr ? `${ipr.slice(0, 30)}...` : 'auto-discover'})`);

  try {
    const t0 = performance.now();
    await setupMixTunnel(opts);
    const ms = (performance.now() - t0).toFixed(0);
    display(`tunnel ready in ${ms} ms`, 'green');
    statusEl.textContent = `Connected (${ms} ms)`;
    statusEl.style.color = 'green';
    setTunnelButtonsEnabled(true);
    ($('btn-disconnect') as HTMLButtonElement).disabled = false;
  } catch (e) {
    const msg = String(e);
    display(`setupMixTunnel failed: ${msg}`, 'red');
    statusEl.textContent = `Failed: ${msg}`;
    statusEl.style.color = 'red';
    statusEl.title = msg;
    ($('btn-setup') as HTMLButtonElement).disabled = false;
  }
});

$('btn-disconnect').addEventListener('click', async () => {
  display('Disconnecting...');
  try {
    await disconnectMixTunnel();
    display('Disconnected', 'green');
    $('tunnel-status').textContent = 'Disconnected';
    $('tunnel-status').style.color = 'gray';
    setTunnelButtonsEnabled(false);
    ($('btn-disconnect') as HTMLButtonElement).disabled = true;
    // Tunnel uses OnceLock semantics — no re-setup without page reload.
    ($('btn-setup') as HTMLButtonElement).disabled = true;
  } catch (e) {
    display(`Disconnect failed: ${e}`, 'red');
  }
});

// DNS Resolve ========================================================

$('btn-dns-tunnel').addEventListener('click', async () => {
  const host = ($('dns-host') as HTMLInputElement).value.trim();
  if (!host) { logTo('dns-log', 'Hostname is required', 'red'); return; }

  const btn = $('btn-dns-tunnel') as HTMLButtonElement;
  btn.disabled = true;
  logTo('dns-log', `tunnel resolve ${host}`);
  const t0 = performance.now();
  try {
    const ip = await mixDNS(host);
    const ms = (performance.now() - t0).toFixed(0);
    logTo('dns-log', `tunnel ${host} => ${ip} (${ms} ms)`, 'green');
  } catch (e) {
    logTo('dns-log', `tunnel resolve failed: ${e}`, 'red');
  } finally {
    btn.disabled = false;
  }
});

// Browsers expose no raw DNS API; the closest analogue from JS is DoH via
// HTTPS to a public resolver. Google's JSON API is CORS-friendly.
$('btn-dns-clearnet').addEventListener('click', async () => {
  const host = ($('dns-host') as HTMLInputElement).value.trim();
  if (!host) { logTo('dns-log', 'Hostname is required', 'red'); return; }

  logTo('dns-log', `clearnet DoH resolve ${host}`);
  const t0 = performance.now();
  try {
    const resp = await window.fetch(
      `https://dns.google/resolve?name=${encodeURIComponent(host)}&type=A`,
      { mode: 'cors' },
    );
    const json = (await resp.json()) as { Status: number; Answer?: Array<{ type: number; data: string }> };
    const ms = (performance.now() - t0).toFixed(0);

    if (json.Status !== 0) {
      logTo('dns-log', `clearnet DoH error: status=${json.Status} (${ms} ms)`, 'red');
      return;
    }
    const a = json.Answer?.find((r) => r.type === 1);
    if (!a) { logTo('dns-log', `clearnet DoH: no A record (${ms} ms)`, 'orange'); return; }
    logTo('dns-log', `clearnet ${host} => ${a.data} (${ms} ms); visible in DevTools Network`, 'green');
  } catch (e) {
    logTo('dns-log', `clearnet DoH failed: ${e}`, 'red');
  }
});

// GET ===============================================================

// `asImage` is a per-call argument rather than module state because GET
// requests interleave: a preset-fired image fetch may still be awaiting its
// body when a second preset click arrives. Closing over the flag in the
// caller's scope keeps each invocation's intent isolated.
async function getViaTunnel(url: string, asImage: boolean) {
  logTo('get-log', `tunnel GET ${url}`);
  const t0 = performance.now();
  try {
    const resp = await mixFetch(url, fetchInit());
    const ms = (performance.now() - t0).toFixed(0);
    const ct = resp.headers.get('content-type') ?? '?';
    logTo('get-log', `tunnel ${resp.status} ${resp.statusText} (${ms} ms, ${ct})`, 'green');

    if (!resp.ok) {
      const errText = await resp.text();
      logTo('get-log', errText.length > 600 ? `${errText.slice(0, 600)}\n... (${errText.length} bytes total)` : errText);
      return;
    }

    if (asImage) {
      const buf = await resp.arrayBuffer();
      const type = resp.headers.get('content-type') ?? 'image/svg+xml';
      const blobUrl = URL.createObjectURL(new Blob([buf], { type }));
      const img = document.createElement('img');
      img.src = blobUrl;
      img.style.maxWidth = '120px';
      img.style.marginRight = '4px';
      img.title = url;
      $('get-image-output').appendChild(img);
    } else {
      const text = await resp.text();
      logTo('get-log', text.length > 400 ? `${text.slice(0, 400)}\n... (${text.length} bytes total)` : text);
    }
  } catch (e) {
    logTo('get-log', `tunnel GET failed: ${e}`, 'red');
  }
}

$('btn-get-tunnel').addEventListener('click', () => {
  const url = ($('get-url') as HTMLInputElement).value.trim();
  if (!url) { logTo('get-log', 'URL is required', 'red'); return; }
  getViaTunnel(url, false);
});

$('btn-get-clearnet').addEventListener('click', async () => {
  const url = ($('get-url') as HTMLInputElement).value.trim();
  if (!url) { logTo('get-log', 'URL is required', 'red'); return; }
  logTo('get-log', `clearnet GET ${url}`);
  const t0 = performance.now();
  try {
    const resp = await window.fetch(url, { mode: 'cors' });
    const ms = (performance.now() - t0).toFixed(0);
    logTo('get-log', `clearnet ${resp.status} ${resp.statusText} (${ms} ms); visible in DevTools Network`, 'green');
  } catch (e) {
    logTo('get-log', `clearnet fetch failed: ${e}`, 'red');
  }
});

// Preset buttons fill the URL field and immediately tunnel-fetch. The
// `data-render="image"` hint asks the GET handler to render the body as an
// inline <img> rather than logging the response text.
for (const btn of Array.from(document.querySelectorAll<HTMLButtonElement>('button.preset'))) {
  btn.addEventListener('click', () => {
    const url = btn.dataset.url ?? '';
    if (!url) return;
    ($('get-url') as HTMLInputElement).value = url;
    getViaTunnel(url, btn.dataset.render === 'image');
  });
}

// WebSocket =========================================================

let activeWs: MixWebSocket | undefined;
let wsConnectT0 = 0;
const wsSendQueue: number[] = [];

// Burst-mode state: collected silently to avoid log spam during 500-msg runs.
let wsBurstActive = false;
let wsBurstRtts: number[] = [];
let wsBurstExpected = 0;
let wsBurstResolve: (() => void) | null = null;
let wsBurstHashes: string[] = [];
let wsBurstVerified = 0;
let wsBurstMismatches = 0;

function setWsButtonState(state: 'connected' | 'connecting' | 'disconnected') {
  const connected = state === 'connected';
  const connecting = state === 'connecting';
  ($('btn-ws-connect') as HTMLButtonElement).disabled = connected || connecting;
  ($('btn-ws-send') as HTMLButtonElement).disabled = !connected;
  ($('btn-ws-close') as HTMLButtonElement).disabled = !connected;
  ($('btn-ws-burst') as HTMLButtonElement).disabled = !connected;
}

$('btn-ws-connect').addEventListener('click', () => {
  const url = ($('ws-url') as HTMLInputElement).value.trim();
  if (!url) { logTo('ws-log', 'WebSocket URL is required', 'red'); return; }

  // Tear down any prior connection so a rapid double-click doesn't leak it.
  if (activeWs && activeWs.readyState !== 3 /* CLOSED */) activeWs.close();

  const statusEl = $('ws-status');
  statusEl.textContent = 'Connecting...';
  statusEl.style.color = 'orange';
  setWsButtonState('connecting');
  wsSendQueue.length = 0;

  logTo('ws-log', `connecting to ${url}`);
  wsConnectT0 = performance.now();

  const ws = new MixWebSocket(url);
  activeWs = ws;

  ws.addEventListener('open', () => {
    const ms = (performance.now() - wsConnectT0).toFixed(0);
    logTo('ws-log', `connected in ${ms} ms`, 'green');
    statusEl.textContent = `Connected (${ms} ms)`;
    statusEl.style.color = 'green';
    setWsButtonState('connected');
  });

  ws.addEventListener('message', async (ev) => {
    const data = (ev as MessageEvent).data;
    let preview: string;
    let bytes: Uint8Array<ArrayBuffer> | undefined;
    if (typeof data === 'string') {
      preview = data.length <= 200 ? data : `${data.slice(0, 200)}...`;
    } else if (data instanceof ArrayBuffer) {
      bytes = new Uint8Array(data as ArrayBuffer);
      preview = `[binary ${bytes.length} bytes] ${hexPreview(bytes)}`;
    } else {
      preview = `[unknown ${typeof data}]`;
    }

    const rttMs = wsSendQueue.length > 0 ? performance.now() - (wsSendQueue.shift() as number) : null;

    if (wsBurstActive) {
      if (rttMs !== null) wsBurstRtts.push(rttMs);
      // Verify echo content against the recorded send hash.
      if (bytes) {
        const hash = await sha256hex(bytes);
        if (hash === wsBurstHashes[wsBurstVerified + wsBurstMismatches]) wsBurstVerified += 1;
        else wsBurstMismatches += 1;
      }
      if (wsBurstRtts.length >= wsBurstExpected && wsBurstResolve) wsBurstResolve();
      return;
    }

    if (rttMs !== null) logTo('ws-log', `recv (${rttMs.toFixed(0)} ms RTT): ${preview}`, 'green');
    else logTo('ws-log', `recv: ${preview}`, 'green');
  });

  ws.addEventListener('close', (ev) => {
    const ce = ev as CloseEvent;
    logTo('ws-log', `closed: ${ce.code} ${ce.reason}`, 'orange');
    statusEl.textContent = 'Closed';
    statusEl.style.color = 'gray';
    setWsButtonState('disconnected');
    activeWs = undefined;
  });

  ws.addEventListener('error', (ev) => {
    // MixWebSocket attaches a non-standard `.message` to its error events so
    // the playground can surface the underlying cause.
    const msg = (ev as Event & { message?: string }).message ?? '(no detail)';
    logTo('ws-log', `error: ${msg}`, 'red');
    statusEl.textContent = 'Error';
    statusEl.style.color = 'red';
  });
});

$('btn-ws-send').addEventListener('click', async () => {
  if (!activeWs || activeWs.readyState !== 1 /* OPEN */) return;
  const msg = ($('ws-message') as HTMLInputElement).value;
  wsSendQueue.push(performance.now());
  try {
    await activeWs.send(msg);
    logTo('ws-log', `send: ${msg}`);
  } catch (e) {
    logTo('ws-log', `send failed: ${e}`, 'red');
  }
});

$('btn-ws-close').addEventListener('click', async () => {
  if (!activeWs) return;
  logTo('ws-log', 'closing...');
  try {
    await activeWs.close(1000, 'done');
  } catch (e) {
    logTo('ws-log', `close failed: ${e}`, 'red');
  }
});

$('btn-ws-burst').addEventListener('click', async () => {
  if (!activeWs || activeWs.readyState !== 1) return;
  const count = parseInt(($('ws-burst-count') as HTMLInputElement).value, 10);
  const minSize = parseInt(($('ws-burst-min') as HTMLInputElement).value, 10);
  const maxSize = parseInt(($('ws-burst-max') as HTMLInputElement).value, 10);

  if (count < 1 || count > 500) { logTo('ws-log', 'burst count must be 1-500', 'red'); return; }
  if (minSize < 1 || maxSize < minSize) { logTo('ws-log', 'invalid size range', 'red'); return; }

  ($('btn-ws-burst') as HTMLButtonElement).disabled = true;
  ($('btn-ws-send') as HTMLButtonElement).disabled = true;

  // Generate random payloads + pre-compute their hashes for echo verification.
  const payloads: Uint8Array[] = [];
  wsBurstHashes = [];
  let totalBytes = 0;
  for (let i = 0; i < count; i++) {
    const size = minSize + Math.floor(Math.random() * (maxSize - minSize + 1));
    const buf = new Uint8Array(size);
    crypto.getRandomValues(buf);
    payloads.push(buf);
    totalBytes += size;
    // eslint-disable-next-line no-await-in-loop
    wsBurstHashes.push(await sha256hex(buf));
  }

  wsBurstActive = true;
  wsBurstRtts = [];
  wsBurstExpected = count;
  wsBurstVerified = 0;
  wsBurstMismatches = 0;
  wsSendQueue.length = 0;

  logTo('ws-log', `burst: ${count} msgs, ${formatSize(totalBytes)} total`);

  const burstDone = new Promise<void>((resolve) => {
    wsBurstResolve = resolve;
  });

  const t0 = performance.now();
  for (const payload of payloads) {
    wsSendQueue.push(performance.now());
    // eslint-disable-next-line no-await-in-loop
    await activeWs.send(payload);
  }

  await burstDone;
  const totalMs = performance.now() - t0;

  wsBurstActive = false;
  wsBurstResolve = null;

  const rtts = wsBurstRtts.slice().sort((a, b) => a - b);
  const rttMin = rtts[0]?.toFixed(0) ?? '?';
  const rttMax = rtts[rtts.length - 1]?.toFixed(0) ?? '?';
  const rttAvg = rtts.length ? (rtts.reduce((a, b) => a + b, 0) / rtts.length).toFixed(0) : '?';
  const p50 = rtts[Math.floor(rtts.length * 0.5)]?.toFixed(0) ?? '?';
  const p95 = rtts[Math.floor(rtts.length * 0.95)]?.toFixed(0) ?? '?';
  const msgPerSec = (count / (totalMs / 1000)).toFixed(1);

  logTo('ws-log', `burst done: ${count} msgs in ${(totalMs / 1000).toFixed(2)}s (${msgPerSec} msg/s, ${formatRate(totalBytes, totalMs)})`, 'green');
  logTo('ws-log', `verify: ${wsBurstVerified}/${count} OK${wsBurstMismatches > 0 ? `, ${wsBurstMismatches} MISMATCH` : ''}`, wsBurstMismatches === 0 ? 'green' : 'red');
  logTo('ws-log', `RTT: min=${rttMin} avg=${rttAvg} p50=${p50} p95=${p95} max=${rttMax} ms`);

  ($('btn-ws-burst') as HTMLButtonElement).disabled = false;
  ($('btn-ws-send') as HTMLButtonElement).disabled = false;
});

// Stress Test =======================================================

interface SizeProfile { label: string; bytes: number; }
const SIZE_PROFILES: SizeProfile[] = [
  { label: 'tiny', bytes: 128 },
  { label: 'small', bytes: 1024 },
  { label: 'medium', bytes: 10240 },
  { label: 'large', bytes: 102400 },
  { label: 'xlarge', bytes: 1048576 },
];

interface DripProfile { label: string; duration: number; delay: number; bytes: number; }
const buildDripProfiles = (timeoutSec: number): DripProfile[] => [
  { label: 'safe',       duration: Math.round(timeoutSec * 0.5),  delay: 0,                              bytes: 100 },
  { label: 'boundary',   duration: Math.round(timeoutSec * 0.92), delay: 0,                              bytes: 100 },
  { label: 'over',       duration: Math.round(timeoutSec * 1.08), delay: 0,                              bytes: 100 },
  { label: 'slow-start', duration: Math.round(timeoutSec * 0.83), delay: Math.round(timeoutSec * 0.17),  bytes: 100 },
];

interface StressRequest { id: number; url: string; label: string; }

function generateRequests(count: number, mode: string, timeoutSec: number): StressRequest[] {
  const requests: StressRequest[] = [];
  if (mode === 'uniform') {
    const baseUrl = ($('stress-url') as HTMLInputElement).value.trim();
    for (let i = 1; i <= count; i++) requests.push({ id: i, url: `${baseUrl}${i}`, label: 'uniform' });
  } else if (mode === 'mixed') {
    for (let i = 1; i <= count; i++) {
      const p = SIZE_PROFILES[Math.floor(Math.random() * SIZE_PROFILES.length)];
      requests.push({ id: i, url: `https://httpbin.org/bytes/${p.bytes}`, label: p.label });
    }
  } else if (mode === 'drip') {
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

interface StressResult { id: number; label: string; ok: boolean; elapsed: string; status?: number; textLength?: number; error?: string; }

async function runOneStressRequest(req: StressRequest): Promise<StressResult> {
  const tag = `#${req.id} ${req.label}`;
  const start = performance.now();
  try {
    const resp = await mixFetch(req.url, fetchInit());
    const body = await resp.text();
    const elapsed = ((performance.now() - start) / 1000).toFixed(2);
    logTo('stress-log', `[${tag}] ${resp.status} OK ${elapsed}s (${body.length}B)`, 'green');
    return { id: req.id, label: req.label, ok: true, elapsed, status: resp.status, textLength: body.length };
  } catch (e) {
    const elapsed = ((performance.now() - start) / 1000).toFixed(2);
    logTo('stress-log', `[${tag}] FAIL ${elapsed}s: ${e}`, 'red');
    return { id: req.id, label: req.label, ok: false, elapsed, error: String(e) };
  }
}

$('stress-mode').addEventListener('change', (ev) => {
  const mode = (ev.target as HTMLSelectElement).value;
  $('stress-uniform-opts').style.display = mode === 'uniform' ? 'block' : 'none';
  $('stress-mixed-opts').style.display   = mode === 'mixed'   ? 'block' : 'none';
  $('stress-drip-opts').style.display    = mode === 'drip'    ? 'block' : 'none';
});

$('btn-stress').addEventListener('click', async () => {
  const count = parseInt(($('stress-count') as HTMLInputElement).value, 10);
  const mode = ($('stress-mode') as HTMLSelectElement).value;
  const timeoutSec = parseInt(($('stress-timeout') as HTMLInputElement)?.value || '60', 10);

  const statusEl = $('stress-status');
  ($('btn-stress') as HTMLButtonElement).disabled = true;
  statusEl.textContent = 'Running...';

  const requests = generateRequests(count, mode, timeoutSec);

  if (mode === 'mixed' || mode === 'drip') {
    const breakdown: Record<string, number> = {};
    for (const r of requests) breakdown[r.label] = (breakdown[r.label] || 0) + 1;
    logTo('stress-log', `${count} requests, ${mode} mode, profiles: ${JSON.stringify(breakdown)}`);
  } else {
    logTo('stress-log', `${count} requests, ${mode} mode`);
  }

  const t0 = performance.now();
  const settled = await Promise.allSettled(requests.map((r) => runOneStressRequest(r)));
  const totalSec = ((performance.now() - t0) / 1000).toFixed(2);

  const results: StressResult[] = settled.map((s) =>
    s.status === 'fulfilled' ? s.value : ({ id: -1, label: '?', ok: false, elapsed: '?', error: String(s.reason) } as StressResult),
  );
  const ok = results.filter((r) => r.ok).length;
  const fail = results.filter((r) => !r.ok).length;

  logTo('stress-log', `done: ${ok}/${count} OK, ${fail} failed (${totalSec}s total)`, fail === 0 ? 'green' : 'red');
  if (fail > 0) {
    for (const r of results.filter((r) => !r.ok)) {
      logTo('stress-log', `  FAIL #${r.id} ${r.label} (${r.elapsed}s): ${r.error}`);
    }
  }

  statusEl.textContent = `Done: ${ok}/${count} OK, ${fail} failed (${totalSec}s)`;
  ($('btn-stress') as HTMLButtonElement).disabled = false;
});

// File Download =====================================================

// UCS Cambridge UTF-8 demo file — small, public, character-rich. Good for
// confirming byte-for-byte preservation across the tunnel.
const VERIFY_TEXT_URL = 'https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-demo.txt';

let cachedPdf: ArrayBuffer | null = null;

$('btn-verify-text').addEventListener('click', async () => {
  const statusEl = $('verify-text-status');
  const outputEl = $('verify-text-output') as HTMLPreElement;
  ($('btn-verify-text') as HTMLButtonElement).disabled = true;
  statusEl.textContent = 'Fetching...';
  statusEl.style.color = 'orange';

  const t0 = performance.now();
  try {
    const resp = await mixFetch(VERIFY_TEXT_URL, fetchInit());
    const text = await resp.text();
    const ms = (performance.now() - t0).toFixed(0);
    statusEl.textContent = `${resp.status} OK (${ms} ms, ${text.length} chars)`;
    statusEl.style.color = 'green';
    outputEl.textContent = text;
    outputEl.style.display = 'block';
    logTo('download-log', `UTF-8 demo: ${text.length} chars (${ms} ms)`, 'green');
  } catch (e) {
    statusEl.textContent = `Failed: ${e}`;
    statusEl.style.color = 'red';
    logTo('download-log', `UTF-8 demo failed: ${e}`, 'red');
  } finally {
    ($('btn-verify-text') as HTMLButtonElement).disabled = false;
  }
});

$('btn-verify-pdf').addEventListener('click', async () => {
  const url = ($('download-url') as HTMLInputElement).value.trim();
  if (!url) { logTo('download-log', 'PDF URL is required', 'red'); return; }

  const statusEl = $('verify-pdf-status');
  const outputEl = $('verify-pdf-output');
  ($('btn-verify-pdf') as HTMLButtonElement).disabled = true;
  statusEl.textContent = 'Fetching...';
  statusEl.style.color = 'orange';

  const t0 = performance.now();
  try {
    const resp = await mixFetch(url, fetchInit());
    const buf = await resp.arrayBuffer();
    const ms = (performance.now() - t0).toFixed(0);
    const hash = await sha256hex(buf);

    cachedPdf = buf;
    $('verify-pdf-size').textContent = `${formatSize(buf.byteLength)} (${buf.byteLength} bytes)`;
    $('verify-pdf-sha').textContent = hash;
    outputEl.style.display = 'block';
    ($('btn-save-pdf') as HTMLButtonElement).disabled = false;

    statusEl.textContent = `${resp.status} OK (${ms} ms)`;
    statusEl.style.color = 'green';
    logTo('download-log', `PDF: ${formatSize(buf.byteLength)} (${ms} ms), sha256=${hash.slice(0, 16)}...`, 'green');
  } catch (e) {
    statusEl.textContent = `Failed: ${e}`;
    statusEl.style.color = 'red';
    logTo('download-log', `PDF failed: ${e}`, 'red');
  } finally {
    ($('btn-verify-pdf') as HTMLButtonElement).disabled = false;
  }
});

$('btn-save-pdf').addEventListener('click', () => {
  if (!cachedPdf) return;
  const url = ($('download-url') as HTMLInputElement).value.trim();
  const name = url.split('/').pop() || 'download.pdf';
  saveFile(cachedPdf, name, 'application/pdf');
});

// Page load ==========================================================

window.addEventListener('DOMContentLoaded', () => {
  // Pre-populate clientId so each page load uses a fresh keystore slot.
  ($('opt-client-id') as HTMLInputElement).value = `sdk-${Math.random().toString(36).slice(2, 8)}`;
  display('SDK dev ready. Click setupMixTunnel to connect.');
});
