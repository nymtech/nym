// smolmix-wasm internal-dev test harness
//
// Runs WASM in a Web Worker via Comlink — all mixnet I/O (smoltcp
// polling, TLS crypto, DNS) runs off the main thread, keeping
// the UI responsive during concurrent requests.

import * as Comlink from 'comlink';
import { MixSocket } from './mix-socket.js';

// ---------------------------------------------------------------------------
// Worker setup
// ---------------------------------------------------------------------------

// Create the Web Worker and wait for its "Loaded" signal before
// wrapping with Comlink.  The worker loads WASM lazily (inside
// setupMixTunnel), so this resolves quickly.
//
// Returns both the raw Worker (for MixSocket postMessage) and
// the Comlink proxy (for fetch request/response calls).
function createWorker() {
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./worker.js', import.meta.url));
    worker.addEventListener('error', reject);
    worker.addEventListener('message', (msg) => {
      if (msg.data?.kind === 'Loaded') {
        worker.removeEventListener('error', reject);
        resolve({ worker, api: Comlink.wrap(worker) });
      }
    }, { once: true });
  });
}

// Comlink proxy to the worker — set in the setup handler.
let api = null;

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

const outputEl = document.getElementById('output');

function display(msg, colour) {
  const ts = new Date().toISOString().slice(11, 23);
  const line = document.createElement('div');
  if (colour) line.style.color = colour;
  line.textContent = `[${ts}] ${msg}`;
  outputEl.appendChild(line);
  outputEl.scrollTop = outputEl.scrollHeight;
}

// Hex preview of a Uint8Array/ArrayBuffer — first `maxBytes` shown as hex pairs
function hexPreview(data, maxBytes = 64) {
  const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
  const len = Math.min(bytes.length, maxBytes);
  const hex = Array.from(bytes.slice(0, len), b => b.toString(16).padStart(2, '0')).join(' ');
  return bytes.length > maxBytes ? `${hex} ...` : hex;
}

// ---------------------------------------------------------------------------
// Response wrapper
// ---------------------------------------------------------------------------

// mixFetch returns { body: Uint8Array, status, statusText, headers }.
// Wrap it in a native Response so callers get .json(), .text(), etc.
function toResponse(raw) {
  return new Response(raw.body, {
    status: raw.status,
    statusText: raw.statusText,
    headers: new Headers(raw.headers),
  });
}

// ---------------------------------------------------------------------------
// Setup / disconnect
// ---------------------------------------------------------------------------

document.getElementById('btn-setup').addEventListener('click', async () => {
  const iprAddress = document.getElementById('ipr-address').value.trim();
  if (!iprAddress) {
    display('IPR address is required', 'red');
    return;
  }

  const statusEl = document.getElementById('tunnel-status');
  document.getElementById('btn-setup').disabled = true;
  statusEl.textContent = 'Starting worker...';
  statusEl.style.color = 'orange';

  try {
    const result = await createWorker();
    api = result.api;
    MixSocket._initWorker(result.worker);
    display('Worker started');
  } catch (e) {
    display(`Worker creation failed: ${e}`, 'red');
    document.getElementById('btn-setup').disabled = false;
    statusEl.textContent = 'Failed';
    statusEl.style.color = 'red';
    return;
  }

  const clientId = document.getElementById('opt-client-id').value;
  const forceTls = document.getElementById('opt-force-tls').checked;
  const disablePoisson = document.getElementById('opt-disable-poisson').checked;
  const disableCover = document.getElementById('opt-disable-cover').checked;

  display(`setupMixTunnel (clientId=${clientId}, IPR: ${iprAddress.slice(0, 30)}...)...`);
  statusEl.textContent = 'Connecting to mixnet...';

  try {
    await api.setupMixTunnel({
      preferredIpr: iprAddress,
      clientId,
      forceTls,
      disablePoissonTraffic: disablePoisson,
      disableCoverTraffic: disableCover,
    });
    display('setupMixTunnel OK — tunnel ready', 'green');
    statusEl.textContent = 'Connected';
    statusEl.style.color = 'green';
    document.getElementById('test-controls').disabled = false;
    document.getElementById('btn-disconnect').disabled = false;
  } catch (e) {
    display(`setupMixTunnel failed: ${e}`, 'red');
    statusEl.textContent = 'Failed';
    statusEl.style.color = 'red';
    document.getElementById('btn-setup').disabled = false;
  }
});

document.getElementById('btn-disconnect').addEventListener('click', async () => {
  display('Disconnecting...');
  try {
    await api.disconnectMixTunnel();
    display('Disconnected', 'green');
    document.getElementById('tunnel-status').textContent = 'Disconnected';
    document.getElementById('tunnel-status').style.color = 'gray';
    document.getElementById('test-controls').disabled = true;
    document.getElementById('btn-disconnect').disabled = true;
    document.getElementById('btn-setup').disabled = true; // OnceLock — can't reinit
  } catch (e) {
    display(`Disconnect failed: ${e}`, 'red');
  }
});

// ---------------------------------------------------------------------------
// HTTPS GET
// ---------------------------------------------------------------------------

async function doGet(url) {
  display(`GET ${url}`);
  const t0 = performance.now();
  try {
    const raw = await api.mixFetch(url, {});
    const resp = toResponse(raw);
    const ms = (performance.now() - t0).toFixed(0);
    display(`${resp.status} ${resp.statusText} (${ms} ms)`, 'green');

    // Body logged to browser devtools (Rust side) — keep output panel clean
  } catch (e) {
    display(`GET failed: ${e}`, 'red');
  }
}

document.getElementById('btn-https').addEventListener('click', () => {
  doGet(document.getElementById('https-url').value.trim());
});

// ---------------------------------------------------------------------------
// POST
// ---------------------------------------------------------------------------

document.getElementById('btn-post').addEventListener('click', async () => {
  const url = document.getElementById('post-url').value.trim();
  const body = document.getElementById('post-body').value;

  display(`POST ${url}`);
  const t0 = performance.now();
  try {
    const raw = await api.mixFetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    const resp = toResponse(raw);
    const ms = (performance.now() - t0).toFixed(0);
    display(`${resp.status} ${resp.statusText} (${ms} ms)`, 'green');
    await resp.text(); // consume body (logged on Rust side)
  } catch (e) {
    display(`POST failed: ${e}`, 'red');
  }
});

// ---------------------------------------------------------------------------
// Formatting helpers
// ---------------------------------------------------------------------------

function formatSize(bytes) {
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(1) + ' MB';
  if (bytes >= 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return bytes + ' B';
}

function formatRate(bytes, ms) {
  const kbps = (bytes / 1024) / (ms / 1000);
  return kbps.toFixed(1) + ' KB/s';
}


// ---------------------------------------------------------------------------
// WebSocket
// ---------------------------------------------------------------------------

let activeWs = null;
let wsConnectT0 = 0;

// Queue of send timestamps for RTT tracking.
// WebSocket preserves message order, so each recv pops the oldest send.
const wsSendQueue = [];

// Burst state
let wsBurstActive = false;
let wsBurstRtts = [];
let wsBurstExpected = 0;
let wsBurstResolve = null;

function setWsButtonState(state) {
  const connected = state === 'connected';
  const connecting = state === 'connecting';
  document.getElementById('btn-ws-connect').disabled = connected || connecting;
  document.getElementById('btn-ws-send').disabled = !connected;
  document.getElementById('btn-ws-close').disabled = !connected;
  document.getElementById('btn-ws-burst').disabled = !connected;
}

document.getElementById('btn-ws-connect').addEventListener('click', () => {
  const url = document.getElementById('ws-url').value.trim();
  if (!url) {
    display('WebSocket URL is required', 'red');
    return;
  }

  const statusEl = document.getElementById('ws-status');
  statusEl.textContent = 'Connecting...';
  statusEl.style.color = 'orange';
  setWsButtonState('connecting');
  wsSendQueue.length = 0;

  display(`[ws] connecting to ${url}`);
  wsConnectT0 = performance.now();

  const ws = new MixSocket(url);

  ws.onopen = () => {
    const ms = (performance.now() - wsConnectT0).toFixed(0);
    display(`[ws] connected in ${ms} ms (protocol=${ws.protocol || 'none'})`, 'green');
    statusEl.textContent = `Connected (${ms} ms)`;
    statusEl.style.color = 'green';
    setWsButtonState('connected');
  };

  ws.onmessage = (e) => {
    let preview;
    if (typeof e.data === 'string') {
      preview = e.data.length <= 200 ? e.data : e.data.slice(0, 200) + '...';
    } else if (e.data instanceof ArrayBuffer) {
      preview = `[binary ${e.data.byteLength} bytes] ${hexPreview(e.data)}`;
    } else if (e.data instanceof Blob) {
      preview = `[blob ${e.data.size} bytes]`;
    } else {
      preview = `[unknown ${typeof e.data}]`;
    }

    let rttMs = null;
    if (wsSendQueue.length > 0) {
      rttMs = performance.now() - wsSendQueue.shift();
    }

    // During burst: collect RTTs silently, don't log each message
    if (wsBurstActive) {
      if (rttMs != null) wsBurstRtts.push(rttMs);
      if (wsBurstRtts.length >= wsBurstExpected && wsBurstResolve) {
        wsBurstResolve();
      }
      return;
    }

    if (rttMs != null) {
      display(`[ws] recv (${rttMs.toFixed(0)} ms RTT): ${preview}`, 'green');
    } else {
      display(`[ws] recv: ${preview}`, 'green');
    }
  };

  ws.onclose = (e) => {
    display(`[ws] closed: ${e.code} ${e.reason}${e.wasClean ? '' : ' (unclean)'}`, 'orange');
    statusEl.textContent = 'Closed';
    statusEl.style.color = 'gray';
    setWsButtonState('disconnected');
    activeWs = null;
  };

  ws.onerror = () => {
    display('[ws] error', 'red');
    statusEl.textContent = 'Error';
    statusEl.style.color = 'red';
  };

  activeWs = ws;
});

document.getElementById('btn-ws-send').addEventListener('click', () => {
  if (!activeWs || activeWs.readyState !== MixSocket.OPEN) return;
  const msg = document.getElementById('ws-message').value;
  wsSendQueue.push(performance.now());
  activeWs.send(msg);
  display(`[ws] send: ${msg}`);
});

document.getElementById('btn-ws-close').addEventListener('click', () => {
  if (!activeWs) return;
  const t0 = performance.now();
  const origOnclose = activeWs.onclose;
  activeWs.onclose = (e) => {
    const ms = (performance.now() - t0).toFixed(0);
    display(`[ws] closed in ${ms} ms: ${e.code} ${e.reason}${e.wasClean ? '' : ' (unclean)'}`, 'orange');
    document.getElementById('ws-status').textContent = 'Closed';
    document.getElementById('ws-status').style.color = 'gray';
    setWsButtonState('disconnected');
    activeWs = null;
  };
  display('[ws] closing...');
  activeWs.close();
});

// Echo burst — send N random binary payloads, verify echoes, collect RTT stats
document.getElementById('btn-ws-burst').addEventListener('click', async () => {
  if (!activeWs || activeWs.readyState !== MixSocket.OPEN) return;
  const count = parseInt(document.getElementById('ws-burst-count').value, 10);
  const minSize = parseInt(document.getElementById('ws-burst-min').value, 10);
  const maxSize = parseInt(document.getElementById('ws-burst-max').value, 10);

  if (count < 1 || count > 500) {
    display('[ws] burst count must be 1-500', 'red');
    return;
  }
  if (minSize < 1 || maxSize < minSize) {
    display('[ws] invalid size range', 'red');
    return;
  }

  // Switch to arraybuffer mode for binary round-trip verification
  const prevBinaryType = activeWs.binaryType;
  activeWs.binaryType = 'arraybuffer';

  document.getElementById('btn-ws-burst').disabled = true;
  document.getElementById('btn-ws-send').disabled = true;

  // Generate payloads up front
  const payloads = [];
  let totalBytes = 0;
  for (let i = 0; i < count; i++) {
    const size = minSize === maxSize
      ? minSize
      : minSize + Math.floor(Math.random() * (maxSize - minSize + 1));
    const buf = new Uint8Array(size);
    crypto.getRandomValues(buf);
    payloads.push(buf);
    totalBytes += size;
  }

  display(
    `[ws] echo burst: ${count} msgs, ${formatSize(minSize)}-${formatSize(maxSize)} ` +
    `(${formatSize(totalBytes)} total)`,
  );

  wsBurstActive = true;
  wsBurstRtts = [];
  wsBurstExpected = count;

  // Track per-message data for verification
  let received = 0;
  let verified = 0;
  let mismatches = 0;
  const sizes = [];
  let firstRecvHex = null;

  const burstDone = new Promise((resolve) => {
    wsBurstResolve = resolve;

    // Override onmessage for burst — verify echo content
    const origOnmessage = activeWs.onmessage;
    activeWs.onmessage = (e) => {
      let rttMs = null;
      if (wsSendQueue.length > 0) {
        rttMs = performance.now() - wsSendQueue.shift();
        wsBurstRtts.push(rttMs);
      }

      // Verify echo matches sent payload
      const sent = payloads[received];
      const recvBuf = new Uint8Array(e.data);
      sizes.push(recvBuf.byteLength);
      if (firstRecvHex === null) firstRecvHex = hexPreview(recvBuf);

      if (sent && recvBuf.byteLength === sent.byteLength) {
        let match = true;
        for (let j = 0; j < sent.byteLength; j++) {
          if (recvBuf[j] !== sent[j]) { match = false; break; }
        }
        if (match) verified++;
        else mismatches++;
      } else {
        mismatches++;
      }

      received++;
      if (received >= count) {
        activeWs.onmessage = origOnmessage;
        resolve();
      }
    };
  });

  const t0 = performance.now();
  for (let i = 0; i < count; i++) {
    wsSendQueue.push(performance.now());
    activeWs.send(payloads[i]);
  }

  await burstDone;
  const totalMs = performance.now() - t0;

  wsBurstActive = false;
  wsBurstResolve = null;
  activeWs.binaryType = prevBinaryType;

  // RTT stats
  const rtts = wsBurstRtts.slice().sort((a, b) => a - b);
  const rttMin = rtts[0].toFixed(0);
  const rttMax = rtts[rtts.length - 1].toFixed(0);
  const rttAvg = (rtts.reduce((a, b) => a + b, 0) / rtts.length).toFixed(0);
  const p50 = rtts[Math.floor(rtts.length * 0.5)].toFixed(0);
  const p95 = rtts[Math.floor(rtts.length * 0.95)].toFixed(0);
  const msgPerSec = (count / (totalMs / 1000)).toFixed(1);
  const throughput = formatRate(totalBytes, totalMs);

  const verifyColour = mismatches === 0 ? 'green' : 'red';
  display(
    `[ws] burst done: ${count} msgs in ${(totalMs / 1000).toFixed(2)}s ` +
    `(${msgPerSec} msg/s, ${throughput})`,
    'green',
  );
  display(
    `[ws] verify: ${verified}/${count} OK` +
    (mismatches > 0 ? `, ${mismatches} MISMATCH` : ''),
    verifyColour,
  );
  display(`[ws] RTT: min=${rttMin} avg=${rttAvg} p50=${p50} p95=${p95} max=${rttMax} ms`);

  document.getElementById('btn-ws-burst').disabled = false;
  document.getElementById('btn-ws-send').disabled = false;
});

// ---------------------------------------------------------------------------
// Stress test — request generation
// ---------------------------------------------------------------------------

const SIZE_PROFILES = [
  { label: 'tiny',   bytes: 128 },
  { label: 'small',  bytes: 1024 },
  { label: 'medium', bytes: 10240 },
  { label: 'large',  bytes: 102400 },
  { label: 'xlarge', bytes: 1048576 },
];

function buildDripProfiles(timeoutSec) {
  return [
    { label: 'safe',       duration: Math.round(timeoutSec * 0.50), delay: 0, bytes: 100 },
    { label: 'boundary',   duration: Math.round(timeoutSec * 0.92), delay: 0, bytes: 100 },
    { label: 'over',       duration: Math.round(timeoutSec * 1.08), delay: 0, bytes: 100 },
    { label: 'slow-start', duration: Math.round(timeoutSec * 0.83), delay: Math.round(timeoutSec * 0.17), bytes: 100 },
  ];
}

function generateRequests(count, mode, timeoutSec) {
  const requests = [];
  if (mode === 'uniform') {
    const baseUrl = document.getElementById('stress-url').value.trim();
    for (let i = 1; i <= count; i++) {
      requests.push({ id: i, url: `${baseUrl}${i}`, label: 'uniform' });
    }
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

// ---------------------------------------------------------------------------
// Stress test — execution
// ---------------------------------------------------------------------------

async function runOneStressRequest(req) {
  const tag = `#${req.id} ${req.label}`;
  const start = performance.now();
  try {
    const raw = await api.mixFetch(req.url, {});
    const resp = toResponse(raw);
    const body = await resp.text();
    const elapsed = ((performance.now() - start) / 1000).toFixed(2);
    display(`[${tag}] ${resp.status} OK ${elapsed}s (${body.length}B)`, 'green');

    return { id: req.id, label: req.label, ok: true, status: resp.status, elapsed, textLength: body.length };
  } catch (e) {
    const elapsed = ((performance.now() - start) / 1000).toFixed(2);
    display(`[${tag}] FAIL ${elapsed}s: ${e}`, 'red');
    return { id: req.id, label: req.label, ok: false, elapsed, error: String(e) };
  }
}

document.getElementById('btn-stress').addEventListener('click', async () => {
  const count = parseInt(document.getElementById('stress-count').value, 10);
  const mode = document.getElementById('stress-mode').value;
  const timeoutSec = parseInt((document.getElementById('stress-timeout') || { value: '60' }).value, 10);

  const statusEl = document.getElementById('stress-status');
  document.getElementById('btn-stress').disabled = true;
  statusEl.textContent = 'Running...';

  const requests = generateRequests(count, mode, timeoutSec);

  if (mode === 'mixed' || mode === 'drip') {
    const breakdown = {};
    for (const r of requests) breakdown[r.label] = (breakdown[r.label] || 0) + 1;
    display(`Stress test: ${count} requests, ${mode} mode, profiles: ${JSON.stringify(breakdown)}`);
  } else {
    display(`Stress test: ${count} requests, ${mode} mode`);
  }

  const t0 = performance.now();

  // All requests fire concurrently — the worker handles them in parallel
  const settled = await Promise.allSettled(requests.map(r => runOneStressRequest(r)));

  const totalSec = ((performance.now() - t0) / 1000).toFixed(2);
  const results = settled.map(s => s.status === 'fulfilled' ? s.value : { ok: false, error: s.reason });
  const ok = results.filter(r => r.ok).length;
  const fail = results.filter(r => !r.ok).length;

  const colour = fail === 0 ? 'green' : 'red';
  display(`Stress test done: ${ok}/${count} OK, ${fail} failed (${totalSec}s total)`, colour);

  if (fail > 0) {
    for (const r of results.filter(r => !r.ok)) {
      display(`  FAIL #${r.id} ${r.label} (${r.elapsed}s): ${r.error}`);
    }
  }

  statusEl.textContent = `Done: ${ok}/${count} OK, ${fail} failed (${totalSec}s)`;
  document.getElementById('btn-stress').disabled = false;
});

// ---------------------------------------------------------------------------
// Stress test — mode selector
// ---------------------------------------------------------------------------

document.getElementById('stress-mode').addEventListener('change', function () {
  document.getElementById('stress-uniform-opts').style.display = this.value === 'uniform' ? 'block' : 'none';
  document.getElementById('stress-mixed-opts').style.display = this.value === 'mixed' ? 'block' : 'none';
  document.getElementById('stress-drip-opts').style.display = this.value === 'drip' ? 'block' : 'none';
});

// ---------------------------------------------------------------------------
// File download
// ---------------------------------------------------------------------------

const VERIFY_TEXT_URL = 'https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-demo.txt';


async function sha256hex(bytes) {
  const hash = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(hash), b => b.toString(16).padStart(2, '0')).join('');
}

// Trigger a browser download (Save As) for an ArrayBuffer
function saveFile(buf, filename, mimeType) {
  const blob = new Blob([buf], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

// Cached ArrayBuffer for Save button
let cachedPdf = null;

// -- Text: UTF-8 demo --

async function verifyText() {
  const statusEl = document.getElementById('verify-text-status');
  const outputEl = document.getElementById('verify-text-output');
  document.getElementById('btn-verify-text').disabled = true;
  statusEl.textContent = 'Fetching...';
  statusEl.style.color = 'orange';

  const t0 = performance.now();
  try {
    const raw = await api.mixFetch(VERIFY_TEXT_URL, {});
    const resp = toResponse(raw);
    const text = await resp.text();
    const ms = (performance.now() - t0).toFixed(0);

    statusEl.textContent = `${formatSize(text.length)} in ${ms} ms`;
    statusEl.style.color = 'green';
    outputEl.style.display = 'block';
    outputEl.textContent = text;
    display(`[verify] UTF-8 demo: ${formatSize(text.length)} in ${ms} ms`, 'green');
  } catch (e) {
    statusEl.textContent = `Failed: ${e}`;
    statusEl.style.color = 'red';
    display(`[verify] UTF-8 demo FAILED: ${e}`, 'red');
  }
  document.getElementById('btn-verify-text').disabled = false;
}

// -- File download (configurable URL) --

async function fetchFile() {
  const url = document.getElementById('download-url').value.trim();
  if (!url) {
    display('Download URL is required', 'red');
    return;
  }

  const statusEl = document.getElementById('verify-pdf-status');
  const outputEl = document.getElementById('verify-pdf-output');
  document.getElementById('btn-verify-pdf').disabled = true;
  document.getElementById('btn-save-pdf').disabled = true;
  cachedPdf = null;
  statusEl.textContent = 'Fetching...';
  statusEl.style.color = 'orange';

  const t0 = performance.now();
  try {
    const raw = await api.mixFetch(url, {});
    const resp = toResponse(raw);
    const buf = await resp.arrayBuffer();
    const ms = (performance.now() - t0).toFixed(0);
    const hash = await sha256hex(buf);

    document.getElementById('verify-pdf-size').textContent =
      `${buf.byteLength.toLocaleString()} bytes`;
    document.getElementById('verify-pdf-sha').textContent = hash;

    statusEl.textContent = `${formatSize(buf.byteLength)} in ${(parseFloat(ms) / 1000).toFixed(1)}s`;
    statusEl.style.color = 'green';
    outputEl.style.display = 'block';

    cachedPdf = buf;
    document.getElementById('btn-save-pdf').disabled = false;

    display(
      `[download] ${formatSize(buf.byteLength)} in ${(parseFloat(ms) / 1000).toFixed(1)}s ` +
      `(${formatRate(buf.byteLength, parseFloat(ms))}) — SHA-256: ${hash.slice(0, 16)}...`,
      'green'
    );
  } catch (e) {
    statusEl.textContent = `Failed: ${e}`;
    statusEl.style.color = 'red';
    display(`[download] FAILED: ${e}`, 'red');
  }
  document.getElementById('btn-verify-pdf').disabled = false;
}


// Event listeners

document.getElementById('btn-verify-text').addEventListener('click', verifyText);
document.getElementById('btn-verify-pdf').addEventListener('click', fetchFile);

document.getElementById('btn-save-pdf').addEventListener('click', () => {
  if (!cachedPdf) return;
  // Extract filename from URL, fall back to 'download'
  const url = document.getElementById('download-url').value.trim();
  const filename = url.split('/').pop()?.split('?')[0] || 'download';
  saveFile(cachedPdf, filename, 'application/octet-stream');
});

document.getElementById('btn-verify-all').addEventListener('click', async () => {
  const statusEl = document.getElementById('verify-all-status');
  statusEl.textContent = 'Running...';
  statusEl.style.color = 'orange';
  display('[download] running both downloads...');

  const t0 = performance.now();
  await Promise.allSettled([verifyText(), fetchFile()]);
  const totalMs = (performance.now() - t0).toFixed(0);

  statusEl.textContent = `Done in ${(parseFloat(totalMs) / 1000).toFixed(1)}s`;
  statusEl.style.color = 'green';
  display(`[download] both complete in ${(parseFloat(totalMs) / 1000).toFixed(1)}s`, 'green');
});

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

// Randomise client ID on each page load for clean state
document.getElementById('opt-client-id').value =
  'smolmix-' + Math.random().toString(36).slice(2, 8);

display('smolmix-wasm dev ready (worker mode). Enter an IPR address and click setupMixTunnel.');
