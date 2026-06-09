// Single interactive playground for the mix-* TypeScript SDK, modelled on
// wasm/smolmix/internal-dev but driving the published @nymproject/mix-* packages
// and trimmed/adapted for a docs audience. One shared tunnel, several sections
// (DNS, GET, WebSocket, stress, download), each with a verbose timeline log and
// and, where it teaches something, a tunnel-vs-clearnet comparison.
//
// The package import is dynamic (see ./lib.ts loadModules) so the multi-MB wasm
// loads only when the visitor clicks Setup, not on page render.

import React, { useCallback, useEffect, useRef, useState } from 'react';
import {
  loadModules,
  randomClientId,
  clampSurbs,
  formatSize,
  formatRate,
  hexPreview,
  sha256hex,
  saveFile,
  dohResolve,
  generateRequests,
  type PlaygroundMods,
  type MixWebSocketLike,
  type SetupOpts,
} from './lib';
import {
  useLogs,
  LogPanel,
  StatusText,
  Spinner,
  Button,
  box,
  row,
  input,
  num,
  legend,
  sub,
  type Status,
} from './ui';

const VERIFY_TEXT_URL = 'https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-demo.txt';

// Default IPR exit for the playground. Users can switch to auto-discovery
// with the "Use random IPR" toggle.
const DEFAULT_IPR =
  '6B6iuWX4bQP4GVA4Yq7XmZencaaGw6BaPY6xJWYSwsbF.6g6LRx1fgU2Q2A4ZPKonYHtfBARh1GPMe1LtXk6vpRR8@q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1';

function eqBytes(a: Uint8Array, b: Uint8Array): boolean {
  if (a.byteLength !== b.byteLength) return false;
  for (let i = 0; i < a.byteLength; i++) if (a[i] !== b[i]) return false;
  return true;
}

export function MixPlayground() {
  const { log, lines } = useLogs();
  const [mods, setMods] = useState<PlaygroundMods | null>(null);
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false); // setup/disconnect in flight
  const [tunnelStatus, setTunnelStatus] = useState<Status>({ text: 'Not started', colour: 'gray' });

  // Connection form.
  const [useRandomIpr, setUseRandomIpr] = useState(false);
  const [iprAddress, setIprAddress] = useState(DEFAULT_IPR);
  const [clientId, setClientId] = useState('');
  const [forceTls, setForceTls] = useState(true);
  const [disablePoisson, setDisablePoisson] = useState(false);
  const [disableCover, setDisableCover] = useState(false);
  const [openSurbs, setOpenSurbs] = useState(10); // matches SurbsConfig::default (ipr.rs)
  const [dataSurbs, setDataSurbs] = useState(2); // matches SurbsConfig::default (ipr.rs)
  const [primaryDns, setPrimaryDns] = useState('');
  const [fallbackDns, setFallbackDns] = useState('');
  const [debug, setDebug] = useState(true);

  // Section inputs.
  const [dnsHost, setDnsHost] = useState('example.com');
  const [getUrl, setGetUrl] = useState('https://httpbin.org/get');
  const [wsUrl, setWsUrl] = useState('wss://echo.websocket.org');
  const [wsMessage, setWsMessage] = useState('Hello from the mixnet!');
  const [wsStatus, setWsStatus] = useState<Status>({ text: 'Not connected', colour: 'gray' });
  const [wsConnected, setWsConnected] = useState(false);
  const [burstCount, setBurstCount] = useState(10);
  const [burstMin, setBurstMin] = useState(64);
  const [burstMax, setBurstMax] = useState(1024);
  const [burstBusy, setBurstBusy] = useState(false);
  const [stressCount, setStressCount] = useState(10);
  const [stressMode, setStressMode] = useState<'uniform' | 'mixed' | 'drip'>('mixed');
  const [stressUrl, setStressUrl] = useState('https://jsonplaceholder.typicode.com/posts/');
  const [stressTimeout, setStressTimeout] = useState(60);
  const [stressBusy, setStressBusy] = useState(false);
  const [stressStatus, setStressStatus] = useState<Status>({ text: '' });
  const [downloadUrl, setDownloadUrl] = useState(
    'https://web.cs.ucdavis.edu/~rogaway/papers/moral-fn.pdf',
  );
  const [textBusy, setTextBusy] = useState(false);
  const [textStatus, setTextStatus] = useState<Status>({ text: '' });
  const [textOutput, setTextOutput] = useState<string | null>(null);
  const [pdfBusy, setPdfBusy] = useState(false);
  const [pdfStatus, setPdfStatus] = useState<Status>({ text: '' });
  const [pdfInfo, setPdfInfo] = useState<{ size: number; hash: string } | null>(null);
  const [filePreview, setFilePreview] = useState<{ url: string; isImage: boolean } | null>(null);
  const [bothStatus, setBothStatus] = useState<Status>({ text: '' });

  const wsRef = useRef<MixWebSocketLike | null>(null);
  const wsSendQueue = useRef<number[]>([]);
  const burstRef = useRef<{
    payloads: Uint8Array[];
    received: number;
    verified: number;
    mismatches: number;
    rtts: number[];
    expected: number;
    resolve: () => void;
  } | null>(null);
  const cachedPdf = useRef<ArrayBuffer | null>(null);

  // Generate the client id after mount (not at render) to keep SSG and client
  // hydration in agreement; see randomClientId in ./lib.
  useEffect(() => {
    setClientId((c) => c || randomClientId());
  }, []);

  // Revoke the previous object URL when the download changes or on unmount.
  useEffect(() => () => { if (filePreview) URL.revokeObjectURL(filePreview.url); }, [filePreview]);

  // Connection -------------------------------------------------------------

  async function setup() {
    setBusy(true);
    const cid = clientId || randomClientId();
    if (cid !== clientId) setClientId(cid);
    setTunnelStatus({ text: 'Loading wasm...', colour: 'orange' });
    let m = mods;
    try {
      if (!m) {
        const t0 = performance.now();
        m = await loadModules();
        setMods(m);
        log('master', `Modules loaded (${(performance.now() - t0).toFixed(0)} ms)`);
      }
    } catch (e) {
      setTunnelStatus({ text: 'Failed to load wasm', colour: 'red' });
      log('master', `module load failed: ${e}`, 'red');
      setBusy(false);
      return;
    }

    if (!useRandomIpr && !iprAddress.trim()) {
      setTunnelStatus({ text: "IPR address required (or check 'random')", colour: 'red' });
      setBusy(false);
      return;
    }

    const opts: SetupOpts = {
      ...(useRandomIpr ? {} : { preferredIpr: iprAddress.trim() }),
      clientId: cid,
      forceTls,
      disablePoissonTraffic: disablePoisson,
      disableCoverTraffic: disableCover,
      openReplySurbs: clampSurbs(openSurbs),
      dataReplySurbs: clampSurbs(dataSurbs),
      primaryDns: primaryDns.trim() || undefined,
      fallbackDns: fallbackDns.trim() || undefined,
      debug,
    };
    log(
      'master',
      `setupMixTunnel (clientId=${cid}, IPR: ${
        useRandomIpr ? 'auto-discover' : iprAddress.trim().slice(0, 28) + '...'
      })`,
    );
    setTunnelStatus({ text: 'Connecting to mixnet...', colour: 'orange' });
    // The gateway/IPR/smoltcp detail is printed by the Rust client straight to
    // the worker's console; it can't be forwarded to this panel. Point the user
    // there rather than silently dropping it.
    log(
      'master',
      debug
        ? 'Connecting... (gateway, IPR discovery and smoltcp logs are in the browser console)'
        : 'Connecting... (tick "Verbose transport logs" for the gateway/IPR detail in the console)',
      'gray',
    );

    try {
      const t0 = performance.now();
      const st = await m.getTunnelState();
      if (st.state === 'ready') {
        log('master', 'Tunnel already up; reusing it.', 'green');
      } else {
        await m.setupMixTunnel(opts);
        log('master', `setupMixTunnel OK: tunnel ready in ${((performance.now() - t0) / 1000).toFixed(1)}s`, 'green');
      }
      const final = await m.getTunnelState();
      log('master', `tunnel state: ${final.state}${final.reason ? ` (${final.reason})` : ''}`);
      setConnected(true);
      setTunnelStatus({ text: 'Connected', colour: 'green' });
    } catch (e) {
      const msg = String(e);
      if (/already initialised/.test(msg)) {
        setTunnelStatus({ text: 'Tunnel spent; reload the page to reconnect', colour: 'red' });
        log('master', 'tunnel already initialised but not ready; reload the page', 'red');
      } else {
        setTunnelStatus({ text: `Failed: ${msg}`, colour: 'red' });
        log('master', `setupMixTunnel failed: ${msg}`, 'red');
      }
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    if (!mods) return;
    setBusy(true);
    log('master', 'Disconnecting...');
    try {
      await mods.disconnectMixTunnel();
      log('master', 'Disconnected. Reload the page to reconnect (the wasm tunnel is one-shot).', 'green');
      setConnected(false);
      setTunnelStatus({ text: 'Disconnected', colour: 'gray' });
    } catch (e) {
      log('master', `disconnect failed: ${e}`, 'red');
    } finally {
      setBusy(false);
    }
  }

  // DNS --------------------------------------------------------------------

  async function dnsTunnel() {
    if (!mods) return;
    const h = dnsHost.trim();
    if (!h) return log('dns', 'Hostname is required', 'red');
    log('dns', `tunnel resolve ${h}`);
    const t0 = performance.now();
    try {
      const ip = await mods.mixDNS(h);
      log('dns', `tunnel ${h} => ${ip} (${(performance.now() - t0).toFixed(0)} ms)`, 'green');
    } catch (e) {
      log('dns', `tunnel resolve failed: ${e}`, 'red');
    }
  }

  async function dnsClearnet() {
    const h = dnsHost.trim();
    if (!h) return log('dns', 'Hostname is required', 'red');
    log('dns', `clearnet DoH resolve ${h}`);
    const t0 = performance.now();
    try {
      const ip = await dohResolve(h);
      log(
        'dns',
        `clearnet ${h} => ${ip} (${(performance.now() - t0).toFixed(0)} ms); visible in DevTools Network`,
        'green',
      );
    } catch (e) {
      log('dns', `clearnet DoH failed: ${e}`, 'red');
    }
  }

  // GET --------------------------------------------------------------------

  async function getTunnel() {
    if (!mods) return;
    const u = getUrl.trim();
    if (!u) return log('get', 'URL is required', 'red');
    log('get', `tunnel GET ${u}`);
    const t0 = performance.now();
    try {
      const resp = await mods.mixFetch(u, {});
      log('get', `tunnel ${resp.status} ${resp.statusText} (${(performance.now() - t0).toFixed(0)} ms)`, 'green');
    } catch (e) {
      log('get', `tunnel GET failed: ${e}`, 'red');
    }
  }

  async function getClearnet() {
    const u = getUrl.trim();
    if (!u) return log('get', 'URL is required', 'red');
    log('get', `clearnet GET ${u}`);
    const t0 = performance.now();
    try {
      const resp = await window.fetch(u, { mode: 'cors' });
      log(
        'get',
        `clearnet ${resp.status} ${resp.statusText} (${(performance.now() - t0).toFixed(0)} ms); visible in DevTools Network`,
        'green',
      );
    } catch (e) {
      log('get', `clearnet fetch failed: ${e}`, 'red');
    }
  }

  // WebSocket --------------------------------------------------------------

  const onWsMessage = useCallback(
    (ev: Event) => {
      const e = ev as MessageEvent;
      let rtt: number | null = null;
      if (wsSendQueue.current.length) rtt = performance.now() - (wsSendQueue.current.shift() as number);

      const b = burstRef.current;
      if (b) {
        if (rtt != null) b.rtts.push(rtt);
        const recvBuf = new Uint8Array(e.data as ArrayBuffer);
        const sent = b.payloads[b.received];
        if (sent && eqBytes(recvBuf, sent)) b.verified++;
        else b.mismatches++;
        b.received++;
        if (b.received >= b.expected) b.resolve();
        return;
      }

      const data = e.data;
      let preview: string;
      if (typeof data === 'string') preview = data.length <= 200 ? data : data.slice(0, 200) + '...';
      else if (data instanceof ArrayBuffer) preview = `[binary ${data.byteLength} bytes] ${hexPreview(data)}`;
      else preview = `[${typeof data}]`;
      log('ws', rtt != null ? `recv (${rtt.toFixed(0)} ms RTT): ${preview}` : `recv: ${preview}`, 'green');
    },
    [log],
  );

  async function wsConnect() {
    if (!mods) return;
    const url = wsUrl.trim();
    if (!url) return log('ws', 'WebSocket URL is required', 'red');
    if (wsRef.current && wsRef.current.readyState !== 3) await wsRef.current.close().catch(() => {});

    setWsStatus({ text: 'Connecting...', colour: 'orange' });
    wsSendQueue.current = [];
    log('ws', `connecting to ${url}`);
    const t0 = performance.now();

    const ws = new mods.MixWebSocket(url);
    ws.addEventListener('message', onWsMessage);
    ws.addEventListener('close', (ev) => {
      const e = ev as CloseEvent;
      log('ws', `closed: ${e.code} ${e.reason || ''}${e.wasClean ? '' : ' (unclean)'}`, 'orange');
      setWsStatus({ text: 'Closed', colour: 'gray' });
      setWsConnected(false);
      wsRef.current = null;
    });
    ws.addEventListener('error', () => {
      log('ws', 'error', 'red');
      setWsStatus({ text: 'Error', colour: 'red' });
    });

    try {
      await ws.opened();
      const ms = (performance.now() - t0).toFixed(0);
      log('ws', `connected in ${ms} ms (protocols=${ws.protocols.join(',') || 'none'})`, 'green');
      setWsStatus({ text: `Connected (${ms} ms)`, colour: 'green' });
      setWsConnected(true);
      wsRef.current = ws;
    } catch (e) {
      log('ws', `connect failed: ${e}`, 'red');
      setWsStatus({ text: 'Error', colour: 'red' });
    }
  }

  async function wsSend() {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return;
    wsSendQueue.current.push(performance.now());
    await ws.send(wsMessage);
    log('ws', `send: ${wsMessage}`);
  }

  async function wsClose() {
    const ws = wsRef.current;
    if (!ws) return;
    log('ws', 'closing...');
    await ws.close(1000, 'user requested');
  }

  async function wsBurst() {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return;
    if (burstCount < 1 || burstCount > 500) return log('ws', 'burst count must be 1-500', 'red');
    if (burstMin < 1 || burstMax < burstMin) return log('ws', 'invalid size range', 'red');

    const payloads: Uint8Array[] = [];
    let totalBytes = 0;
    for (let i = 0; i < burstCount; i++) {
      const size = burstMin === burstMax ? burstMin : burstMin + Math.floor(Math.random() * (burstMax - burstMin + 1));
      const buf = new Uint8Array(size);
      crypto.getRandomValues(buf);
      payloads.push(buf);
      totalBytes += size;
    }
    log('ws', `echo burst: ${burstCount} msgs, ${formatSize(burstMin)}-${formatSize(burstMax)} (${formatSize(totalBytes)} total)`);
    setBurstBusy(true);

    const done = new Promise<void>((resolve) => {
      burstRef.current = { payloads, received: 0, verified: 0, mismatches: 0, rtts: [], expected: burstCount, resolve };
    });
    const t0 = performance.now();
    for (let i = 0; i < burstCount; i++) {
      wsSendQueue.current.push(performance.now());
      ws.send(payloads[i]); // fire in order; Comlink preserves FIFO to the worker
    }
    await done;
    const totalMs = performance.now() - t0;
    const b = burstRef.current!;
    burstRef.current = null;

    const rtts = b.rtts.slice().sort((a, c) => a - c);
    const pick = (q: number) => (rtts.length ? rtts[Math.min(rtts.length - 1, Math.floor(rtts.length * q))].toFixed(0) : 'n/a');
    const avg = rtts.length ? (rtts.reduce((a, c) => a + c, 0) / rtts.length).toFixed(0) : 'n/a';
    const msgPerSec = (burstCount / (totalMs / 1000)).toFixed(1);

    log('ws', `burst done: ${burstCount} msgs in ${(totalMs / 1000).toFixed(2)}s (${msgPerSec} msg/s, ${formatRate(totalBytes, totalMs)})`, 'green');
    log('ws', `verify: ${b.verified}/${burstCount} OK${b.mismatches ? `, ${b.mismatches} MISMATCH` : ''}`, b.mismatches === 0 ? 'green' : 'red');
    log('ws', `RTT: min=${pick(0)} avg=${avg} p50=${pick(0.5)} p95=${pick(0.95)} max=${pick(1)} ms`);
    setBurstBusy(false);
  }

  // Stress -----------------------------------------------------------------

  async function oneStress(req: { id: number; url: string; label: string }) {
    const start = performance.now();
    try {
      const resp = await mods!.mixFetch(req.url, {});
      const body = await resp.text();
      const el = ((performance.now() - start) / 1000).toFixed(2);
      log('stress', `[#${req.id} ${req.label}] ${resp.status} OK ${el}s (${body.length}B)`, 'green');
      return { ok: true, id: req.id, label: req.label };
    } catch (e) {
      const el = ((performance.now() - start) / 1000).toFixed(2);
      log('stress', `[#${req.id} ${req.label}] FAIL ${el}s: ${e}`, 'red');
      return { ok: false, id: req.id, label: req.label };
    }
  }

  async function runStress() {
    if (!mods) return;
    setStressBusy(true);
    setStressStatus({ text: 'Running...', colour: 'orange' });
    const reqs = generateRequests(stressCount, stressMode, stressTimeout, stressUrl.trim());
    if (stressMode !== 'uniform') {
      const bd: Record<string, number> = {};
      reqs.forEach((r) => (bd[r.label] = (bd[r.label] || 0) + 1));
      log('stress', `${stressCount} requests, ${stressMode} mode, profiles: ${JSON.stringify(bd)}`);
    } else {
      log('stress', `${stressCount} requests, uniform mode`);
    }
    const t0 = performance.now();
    const settled = await Promise.allSettled(reqs.map((r) => oneStress(r)));
    const totalSec = ((performance.now() - t0) / 1000).toFixed(2);
    const ok = settled.filter((s) => s.status === 'fulfilled' && s.value.ok).length;
    const fail = stressCount - ok;
    log('stress', `done: ${ok}/${stressCount} OK, ${fail} failed (${totalSec}s total)`, fail === 0 ? 'green' : 'red');
    setStressStatus({ text: `Done: ${ok}/${stressCount} OK, ${fail} failed (${totalSec}s)`, colour: fail === 0 ? 'green' : 'red' });
    setStressBusy(false);
  }

  // Download ---------------------------------------------------------------

  async function verifyText() {
    if (!mods) return;
    setTextBusy(true);
    setTextStatus({ text: 'Fetching...', colour: 'orange' });
    log('download', `GET ${VERIFY_TEXT_URL} over the tunnel... (live transport logs in the browser console)`, 'orange');
    const t0 = performance.now();
    try {
      const resp = await mods.mixFetch(VERIFY_TEXT_URL, {});
      const text = await resp.text();
      const ms = (performance.now() - t0).toFixed(0);
      setTextStatus({ text: `${formatSize(text.length)} in ${ms} ms`, colour: 'green' });
      setTextOutput(text);
      log('download', `UTF-8 demo: ${formatSize(text.length)} in ${ms} ms`, 'green');
    } catch (e) {
      setTextStatus({ text: `Failed: ${e}`, colour: 'red' });
      log('download', `UTF-8 demo FAILED: ${e}`, 'red');
    } finally {
      setTextBusy(false);
    }
  }

  async function fetchFile() {
    if (!mods) return;
    const url = downloadUrl.trim();
    if (!url) return log('download', 'Download URL is required', 'red');
    setPdfBusy(true);
    cachedPdf.current = null;
    setPdfInfo(null);
    setFilePreview(null);
    setPdfStatus({ text: 'Fetching...', colour: 'orange' });
    log('download', `GET ${url} over the tunnel... (live transport logs in the browser console)`, 'orange');
    const t0 = performance.now();
    try {
      const resp = await mods.mixFetch(url, {});
      const buf = await resp.arrayBuffer();
      const ms = performance.now() - t0;
      const hash = await sha256hex(buf);
      cachedPdf.current = buf;
      setPdfInfo({ size: buf.byteLength, hash });
      const contentType = resp.headers.get('content-type') || '';
      const isImage = contentType.startsWith('image/') || /\.(jpe?g|png|gif|webp|avif|svg)(\?|$)/i.test(url);
      const objectUrl = URL.createObjectURL(new Blob([buf], contentType ? { type: contentType } : undefined));
      setFilePreview({ url: objectUrl, isImage });
      setPdfStatus({ text: `${formatSize(buf.byteLength)} in ${(ms / 1000).toFixed(1)}s`, colour: 'green' });
      log(
        'download',
        `${formatSize(buf.byteLength)} in ${(ms / 1000).toFixed(1)}s (${formatRate(buf.byteLength, ms)}); SHA-256: ${hash.slice(0, 16)}...`,
        'green',
      );
    } catch (e) {
      setPdfStatus({ text: `Failed: ${e}`, colour: 'red' });
      log('download', `FAILED: ${e}`, 'red');
    } finally {
      setPdfBusy(false);
    }
  }

  function savePdf() {
    const buf = cachedPdf.current;
    if (!buf) return;
    const filename = decodeURIComponent(downloadUrl.trim().split('/').pop()?.split('?')[0] || 'download');
    saveFile(buf, filename, 'application/octet-stream');
  }

  async function runBoth() {
    setBothStatus({ text: 'Running...', colour: 'orange' });
    const t0 = performance.now();
    await Promise.allSettled([verifyText(), fetchFile()]);
    setBothStatus({ text: `Done in ${((performance.now() - t0) / 1000).toFixed(1)}s`, colour: 'green' });
  }

  // Render -----------------------------------------------------------------

  return (
    <div style={{ margin: '1.5rem 0' }}>
      {/* Connection */}
      <div style={box}>
        <div style={legend}>Connection</div>
        <div style={row}>
          <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
            <input type="checkbox" checked={useRandomIpr} onChange={(e) => setUseRandomIpr(e.target.checked)} />
            Use random IPR
          </label>
          <input
            style={input}
            value={iprAddress}
            onChange={(e) => setIprAddress(e.target.value)}
            placeholder="<nym-address of IPR exit node>"
            disabled={useRandomIpr}
          />
        </div>

        <details style={{ margin: '0.5rem 0' }}>
          <summary style={{ cursor: 'pointer', ...sub }}>Advanced options</summary>
          <div style={{ padding: '0.6rem 0' }}>
            <div style={row}>
              <label style={sub}>
                <input type="checkbox" checked={forceTls} onChange={(e) => setForceTls(e.target.checked)} /> Force TLS
              </label>
              <label style={sub}>
                <input type="checkbox" checked={disablePoisson} onChange={(e) => setDisablePoisson(e.target.checked)} /> Disable Poisson traffic
              </label>
              <label style={sub}>
                <input type="checkbox" checked={disableCover} onChange={(e) => setDisableCover(e.target.checked)} /> Disable cover traffic
              </label>
            </div>
            <div style={row}>
              <label style={sub}>Client ID</label>
              <input style={input} value={clientId} onChange={(e) => setClientId(e.target.value)} />
            </div>
            <div style={row}>
              <label style={sub}>Open SURBs</label>
              <input style={num} type="number" min={0} max={50} value={openSurbs} onChange={(e) => setOpenSurbs(+e.target.value)} />
              <label style={sub}>Data SURBs</label>
              <input style={num} type="number" min={0} max={50} value={dataSurbs} onChange={(e) => setDataSurbs(+e.target.value)} />
            </div>
            <div style={row}>
              <label style={sub}>Primary DNS</label>
              <input style={input} value={primaryDns} onChange={(e) => setPrimaryDns(e.target.value)} placeholder="8.8.8.8:53" />
              <label style={sub}>Fallback DNS</label>
              <input style={input} value={fallbackDns} onChange={(e) => setFallbackDns(e.target.value)} placeholder="1.1.1.1:53" />
            </div>
          </div>
        </details>

        <div style={row}>
          <Button onClick={setup} disabled={busy || connected}>
            {busy ? 'Working...' : 'setupMixTunnel'}
          </Button>
          <Button onClick={disconnect} disabled={busy || !connected}>
            disconnectMixTunnel
          </Button>
          <label
            style={sub}
            title="Routes the Rust client's deep [smolmix] logs (gateway, IPR discovery, smoltcp) to the browser console. Set before connecting."
          >
            <input type="checkbox" checked={debug} onChange={(e) => setDebug(e.target.checked)} /> Verbose transport logs → console
          </label>
          <StatusText status={tunnelStatus} />
        </div>
        <LogPanel lines={lines('master')} placeholder="Press setupMixTunnel to bring up the tunnel." />
        <div style={{ ...sub, marginTop: '0.5rem' }}>
          One-shot per page: after <code>disconnectMixTunnel</code> you must reload to reconnect, and each load uses a fresh client identity.
        </div>
        <div style={{ ...sub, marginTop: '0.35rem' }}>
          This timeline shows the API-level events your code sees; the Rust client's deep transport logs (gateway, IPR discovery, smoltcp) go to the browser console behind <strong>Verbose transport logs</strong>.
        </div>
      </div>

      {/* DNS */}
      <div style={box}>
        <div style={legend}>DNS resolve: tunnel vs clearnet</div>
        <div style={row}>
          <input style={input} value={dnsHost} onChange={(e) => setDnsHost(e.target.value)} placeholder="example.com" />
          <Button onClick={dnsTunnel} disabled={!connected}>via tunnel (IPR)</Button>
          <Button onClick={dnsClearnet}>via DoH (clearnet)</Button>
        </div>
        <div style={sub}>The clearnet DoH query appears in DevTools Network; the tunnel resolution does not.</div>
        <div style={sub}>Resolve the same hostname twice: the second answer comes from the in-wasm DNS cache, served locally with no mixnet round-trip.</div>
        <LogPanel lines={lines('dns')} />
      </div>

      {/* GET */}
      <div style={box}>
        <div style={legend}>GET: tunnel vs clearnet</div>
        <div style={row}>
          <input style={input} value={getUrl} onChange={(e) => setGetUrl(e.target.value)} placeholder="https://..." />
          <Button onClick={getTunnel} disabled={!connected}>via tunnel</Button>
          <Button onClick={getClearnet}>via window.fetch</Button>
        </div>
        <div style={sub}>Both buttons request the same URL, but the clearnet one reaches the server from your own IP and the tunnel one from the IPR's exit gateway.</div>
        <div style={sub}>The clearnet button is a normal browser request, so some hosts block it with CORS while the tunnel request to the same URL succeeds; the defaults here are CORS-permissive.</div>
        <div style={sub}>The first tunnel request to a host runs a full TCP + TLS handshake (visible in the browser console with debug logging on). The HTTPS connection is then pooled, so a second request to the same host skips the handshake; the log timings show the difference.</div>
        <LogPanel lines={lines('get')} />
      </div>

      {/* WebSocket */}
      <div style={box}>
        <div style={legend}>WebSocket</div>
        <div style={row}>
          <input style={input} value={wsUrl} onChange={(e) => setWsUrl(e.target.value)} placeholder="wss://..." />
          <Button onClick={wsConnect} disabled={!connected || wsConnected}>Connect</Button>
          <Button onClick={wsClose} disabled={!wsConnected}>Close</Button>
          <StatusText status={wsStatus} />
        </div>
        <div style={row}>
          <input style={input} value={wsMessage} onChange={(e) => setWsMessage(e.target.value)} />
          <Button onClick={wsSend} disabled={!wsConnected || burstBusy}>Send</Button>
        </div>
        <div style={row}>
          <label style={sub}>Echo burst</label>
          <input style={num} type="number" min={1} max={500} value={burstCount} onChange={(e) => setBurstCount(+e.target.value)} />
          <label style={sub}>size</label>
          <input style={num} type="number" min={1} value={burstMin} onChange={(e) => setBurstMin(+e.target.value)} />
          <span style={sub}>–</span>
          <input style={num} type="number" min={1} value={burstMax} onChange={(e) => setBurstMax(+e.target.value)} />
          <span style={sub}>bytes</span>
          <Button onClick={wsBurst} disabled={!wsConnected || burstBusy}>{burstBusy ? 'Bursting...' : 'Send burst'}</Button>
        </div>
        <div style={sub}>Connecting runs a TCP handshake (plus a TLS handshake for wss://) inside the worker, visible in the browser console with debug logging on.</div>
        <LogPanel lines={lines('ws')} />
      </div>

      {/* Stress */}
      <div style={box}>
        <div style={legend}>Stress test</div>
        <div style={row}>
          <label style={sub}>Requests</label>
          <input style={num} type="number" min={1} max={200} value={stressCount} onChange={(e) => setStressCount(+e.target.value)} />
          <label style={sub}>Mode</label>
          <select style={{ ...input, flex: '0 0 9rem' }} value={stressMode} onChange={(e) => setStressMode(e.target.value as typeof stressMode)}>
            <option value="uniform">Uniform</option>
            <option value="mixed">Mixed sizes</option>
            <option value="drip">Slow drip</option>
          </select>
          <Button onClick={runStress} disabled={!connected || stressBusy}>{stressBusy ? 'Running...' : 'Run stress test'}</Button>
          <StatusText status={stressStatus} />
        </div>
        {stressMode === 'uniform' && (
          <div style={row}>
            <label style={sub}>Base URL</label>
            <input style={input} value={stressUrl} onChange={(e) => setStressUrl(e.target.value)} />
          </div>
        )}
        {stressMode === 'mixed' && <div style={sub}>Random mix of 128 B / 1 KB / 10 KB / 100 KB / 1 MB responses (httpbin.org/bytes).</div>}
        {stressMode === 'drip' && (
          <div style={row}>
            <label style={sub}>Timeout (s)</label>
            <input style={num} type="number" min={5} max={300} value={stressTimeout} onChange={(e) => setStressTimeout(+e.target.value)} />
            <span style={sub}>safe / boundary / over / slow-start, relative to this timeout (httpbin.org/drip).</span>
          </div>
        )}
        <div style={sub}>Requests to the same host share one pooled TCP + TLS connection, so only the first pays the handshake cost.</div>
        <LogPanel lines={lines('stress')} />
      </div>

      {/* Download */}
      <div style={box}>
        <div style={legend}>File download</div>
        <div style={row}>
          <Button onClick={verifyText} disabled={!connected || textBusy}>Fetch UTF-8 text</Button>
          {textBusy ? <Spinner label="downloading... see the browser console for progress" /> : <StatusText status={textStatus} />}
        </div>
        {textOutput != null && (
          <pre
            style={{
              maxHeight: 180,
              overflowY: 'auto',
              fontSize: 12,
              whiteSpace: 'pre-wrap',
              background: 'rgba(127,127,127,0.06)',
              border: '1px solid rgba(127,127,127,0.2)',
              borderRadius: 6,
              padding: '0.5rem',
            }}
          >
            {textOutput}
          </pre>
        )}
        <div style={row}>
          <input style={input} value={downloadUrl} onChange={(e) => setDownloadUrl(e.target.value)} />
          <Button onClick={fetchFile} disabled={!connected || pdfBusy}>Fetch file</Button>
          <Button onClick={savePdf} disabled={!pdfInfo}>Save</Button>
          <Button onClick={() => filePreview && window.open(filePreview.url, '_blank')} disabled={!filePreview}>Open in new tab</Button>
          <Button onClick={runBoth} disabled={!connected}>Run both</Button>
          {pdfBusy ? <Spinner label="downloading... see the browser console for progress" /> : <StatusText status={bothStatus} />}
        </div>
        {pdfInfo && (
          <div style={sub}>
            Size: {pdfInfo.size.toLocaleString()} bytes · SHA-256: <code>{pdfInfo.hash}</code>
          </div>
        )}
        {filePreview?.isImage && (
          <img
            src={filePreview.url}
            alt="File downloaded over the mixnet"
            style={{
              maxHeight: 240,
              maxWidth: '100%',
              marginTop: '0.5rem',
              borderRadius: 6,
              border: '1px solid rgba(127,127,127,0.25)',
              display: 'block',
            }}
          />
        )}
        <div style={sub}>Fetches a real file over the tunnel and reports its size and SHA-256. Fetch it twice and the second download reuses the pooled HTTPS connection, skipping the handshake.</div>
        <LogPanel lines={lines('download')} />
      </div>
    </div>
  );
}
