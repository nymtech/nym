// Shared mixnet-tunnel setup panel for the in-docs demos.
//
// Owns the connection lifecycle (setup / disconnect / state) and the options
// surface (IPR pin, SURBs, DNS, timeouts, ...), and hands the parent demo a
// `mixFetch` function once the tunnel is `ready`. Modelled on the playground's
// inline setup section; the demos differ only in what they do with `mixFetch`.
//
// The package import is dynamic so the multi-MB wasm chunk loads only when the
// visitor clicks Connect, not on page render. Everything here is client-only;
// render the demo page with `next/dynamic` + `ssr: false`.

import React, { useEffect, useState } from 'react';
import { Button, LogPanel, StatusText, useLogs, box, row, input, num, sub, legend, type Status } from './ui';

export type MixFetchFn = (url: string, init?: RequestInit) => Promise<Response>;

interface MixFetchModule {
  setupMixTunnel: (opts?: Record<string, unknown>) => Promise<void>;
  disconnectMixTunnel: () => Promise<void>;
  getTunnelState: () => Promise<{ state: string; reason?: string }>;
  mixFetch: MixFetchFn;
}

// Lazy-load the published mix-fetch facade. The literal specifier keeps webpack
// code-splitting the wasm into an async chunk.
async function loadMixFetch(): Promise<MixFetchModule> {
  // @ts-ignore -- @nymproject/mix-fetch resolves at runtime; lazy wasm chunk
  const m = await import('@nymproject/mix-fetch');
  return m as unknown as MixFetchModule;
}

const clampSurbs = (n: number, min: number) => Math.min(50, Math.max(min, n));

// Default IPR exit for the docs demos. Pinned so a demo connects to a known
// exit by default; users can switch to auto-discovery with 'Use random IPR'.
const DEFAULT_IPR =
  '6B6iuWX4bQP4GVA4Yq7XmZencaaGw6BaPY6xJWYSwsbF.6g6LRx1fgU2Q2A4ZPKonYHtfBARh1GPMe1LtXk6vpRR8@q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1';

// Optional gateway pin. Empty = let the client pick a gateway at random. Set it
// to an identity (e.g. 'q2A2cbooyC16YJzvdYaSMH9X3cSiieZNtfBr8cE8Fi1') to force a
// known entry gateway for debugging.
const PINNED_GATEWAY = '';

export function MixTunnelSetup({
  onReady,
  onDisconnect,
  clientIdPrefix = 'docs-demo',
}: {
  onReady: (mixFetch: MixFetchFn) => void;
  onDisconnect?: () => void;
  clientIdPrefix?: string;
}) {
  const { log, lines } = useLogs();
  const [mods, setMods] = useState<MixFetchModule | null>(null);
  const [connected, setConnected] = useState(false);
  const [busy, setBusy] = useState(false);
  // The tunnel is one-shot per page (smolmix OnceLock + single worker), so once
  // it has been torn down, Connect stays disabled until a reload.
  const [terminated, setTerminated] = useState(false);
  const [status, setStatus] = useState<Status>({ text: 'Not started', colour: 'gray' });
  const [showAdvanced, setShowAdvanced] = useState(false);

  // Connection options.
  const [useRandomIpr, setUseRandomIpr] = useState(false);
  const [iprAddress, setIprAddress] = useState(DEFAULT_IPR);
  const [clientId, setClientId] = useState('');
  const [forceTls, setForceTls] = useState(true);
  const [disablePoisson, setDisablePoisson] = useState(false);
  const [disableCover, setDisableCover] = useState(false);
  const [debug, setDebug] = useState(true);
  const [openSurbs, setOpenSurbs] = useState(10);
  const [dataSurbs, setDataSurbs] = useState(2);
  const [primaryDns, setPrimaryDns] = useState('');
  const [fallbackDns, setFallbackDns] = useState('');
  const [dnsTimeout, setDnsTimeout] = useState('');
  const [connectTimeout, setConnectTimeout] = useState('');
  const [maxRedirects, setMaxRedirects] = useState('');
  const [storagePassphrase, setStoragePassphrase] = useState('');

  // Generate the client id after mount (not at render) so SSG and client
  // hydration agree: Math.random at render would differ between the two.
  useEffect(() => {
    setClientId((c) => c || `${clientIdPrefix}-${Math.random().toString(36).slice(2, 8)}`);
  }, [clientIdPrefix]);

  const optInt = (v: string): number | undefined => {
    const n = parseInt(v, 10);
    return Number.isNaN(n) ? undefined : n;
  };
  const optStr = (v: string): string | undefined => v.trim() || undefined;

  async function connect() {
    if (!useRandomIpr && !iprAddress.trim()) {
      setStatus({ text: "IPR address required (or tick 'Use random IPR')", colour: 'red' });
      return;
    }
    setBusy(true);
    setStatus({ text: 'Connecting (building the client, connecting to the IPR exit)...', colour: 'orange' });
    log('tunnel', `Connecting (clientId=${clientId}, IPR: ${useRandomIpr ? 'auto-discover' : iprAddress.trim().slice(0, 28) + '...'}, SURBs open=${openSurbs} data=${dataSurbs})`, 'orange');
    try {
      const m = mods ?? (await loadMixFetch());
      if (!mods) setMods(m);
      // One WASM instance per browser tab, shared across demo pages by the
      // bundler. If another page already brought the tunnel up, reuse it rather
      // than calling setupMixTunnel again (which throws "already initialised").
      const existing = await m.getTunnelState().catch(() => null);
      if (existing && existing.state === 'ready') {
        log('tunnel', 'Tunnel already up from another page; reusing it (its original options apply).', 'green');
      } else {
        await m.setupMixTunnel({
          ...(useRandomIpr ? {} : { preferredIpr: iprAddress.trim() }),
          ...(PINNED_GATEWAY ? { preferredGateway: PINNED_GATEWAY } : {}),
          clientId,
          forceTls,
          disablePoissonTraffic: disablePoisson,
          disableCoverTraffic: disableCover,
          openReplySurbs: clampSurbs(openSurbs, 1),
          dataReplySurbs: clampSurbs(dataSurbs, 0),
          primaryDns: optStr(primaryDns),
          fallbackDns: optStr(fallbackDns),
          dnsTimeoutMs: optInt(dnsTimeout),
          connectTimeoutMs: optInt(connectTimeout),
          maxRedirects: optInt(maxRedirects),
          storagePassphrase: storagePassphrase || undefined,
          debug,
        });
        log('tunnel', 'Tunnel ready', 'green');
      }
      setConnected(true);
      setStatus({ text: 'Connected', colour: 'green' });
      onReady(m.mixFetch);
    } catch (e) {
      const msg = String((e as any)?.message ?? e);
      if (/already initialised/i.test(msg)) {
        log('tunnel', 'Tunnel already initialised in this tab; reload the page if it does not connect.', 'orange');
        setStatus({ text: 'Failed (already initialised, reload)', colour: 'red' });
      } else {
        setStatus({ text: 'Failed', colour: 'red' });
        log('tunnel', `Connection failed: ${msg}`, 'red');
        log('tunnel', "Timeouts and IPR rate-limits are common. Try again, or tick 'Use random IPR' and reload.", 'orange');
      }
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    if (!mods) return;
    setBusy(true);
    log('tunnel', 'Disconnecting...');
    try {
      await mods.disconnectMixTunnel();
      log('tunnel', 'Disconnected. Reload the page to reconnect.', 'green');
      setStatus({ text: 'Disconnected (reload to reconnect)', colour: 'gray' });
    } catch (e) {
      log('tunnel', `Disconnect failed: ${e}`, 'red');
      setStatus({ text: 'Disconnected after error (reload to reconnect)', colour: 'red' });
    } finally {
      // The tunnel is one-shot per page: smolmix uses a OnceLock and the package
      // owns one worker, so there is no fresh-client path without a reload. Keep
      // Connect disabled and say so rather than failing on a second connect.
      setConnected(false);
      setTerminated(true);
      setBusy(false);
      onDisconnect?.();
    }
  }

  return (
    <div style={box}>
      <div style={legend}>Mixnet tunnel</div>
      <div style={row}>
        <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
          <input type="checkbox" checked={useRandomIpr} onChange={(e) => setUseRandomIpr(e.target.checked)} disabled={connected || busy} />
          Use random IPR
        </label>
        <input
          style={input}
          value={iprAddress}
          onChange={(e) => setIprAddress(e.target.value)}
          placeholder="<nym-address of IPR exit node>"
          disabled={useRandomIpr || connected || busy}
        />
      </div>
      <div style={row}>
        <Button onClick={connect} disabled={connected || busy || terminated}>{busy && !connected ? 'Connecting...' : 'Connect to mixnet'}</Button>
        <Button onClick={disconnect} disabled={!connected || busy}>Disconnect</Button>
        <StatusText status={status} />
        <button
          type="button"
          aria-expanded={showAdvanced}
          style={{ ...sub, marginLeft: 'auto', cursor: 'pointer', background: 'none', border: 'none', padding: 0, fontFamily: 'inherit', fontWeight: 'inherit', color: 'inherit' }}
          onClick={() => setShowAdvanced((v) => !v)}
        >
          {showAdvanced ? '▾ advanced' : '▸ advanced'}
        </button>
      </div>

      {showAdvanced && (
        <div style={{ ...row, flexDirection: 'column', alignItems: 'stretch', gap: 6 }}>
          <div style={row}>
            <label style={sub}>client id</label>
            <input style={input} value={clientId} onChange={(e) => setClientId(e.target.value)} disabled={connected || busy} />
          </div>
          <div style={row}>
            <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
              <input type="checkbox" checked={forceTls} onChange={(e) => setForceTls(e.target.checked)} disabled={connected || busy} /> forceTls (WSS to gateway)
            </label>
            <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
              <input type="checkbox" checked={disablePoisson} onChange={(e) => setDisablePoisson(e.target.checked)} disabled={connected || busy} /> disable Poisson
            </label>
            <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
              <input type="checkbox" checked={disableCover} onChange={(e) => setDisableCover(e.target.checked)} disabled={connected || busy} /> disable cover traffic
            </label>
            <label style={{ ...sub, display: 'flex', gap: 6, alignItems: 'center' }}>
              <input type="checkbox" checked={debug} onChange={(e) => setDebug(e.target.checked)} disabled={connected || busy} /> verbose console logs
            </label>
          </div>
          <div style={row}>
            <label style={sub}>open SURBs</label>
            <input style={num} type="number" min={1} value={openSurbs} onChange={(e) => setOpenSurbs(+e.target.value)} disabled={connected || busy} />
            <label style={sub}>data SURBs</label>
            <input style={num} type="number" min={0} value={dataSurbs} onChange={(e) => setDataSurbs(+e.target.value)} disabled={connected || busy} />
          </div>
          <div style={row}>
            <label style={sub}>primary DNS</label>
            <input style={input} value={primaryDns} onChange={(e) => setPrimaryDns(e.target.value)} placeholder="8.8.8.8:53" disabled={connected || busy} />
            <label style={sub}>fallback DNS</label>
            <input style={input} value={fallbackDns} onChange={(e) => setFallbackDns(e.target.value)} placeholder="1.1.1.1:53" disabled={connected || busy} />
          </div>
          <div style={row}>
            <label style={sub}>dns timeout ms</label>
            <input style={num} value={dnsTimeout} onChange={(e) => setDnsTimeout(e.target.value)} placeholder="30000" disabled={connected || busy} />
            <label style={sub}>connect timeout ms</label>
            <input style={num} value={connectTimeout} onChange={(e) => setConnectTimeout(e.target.value)} placeholder="60000" disabled={connected || busy} />
            <label style={sub}>max redirects</label>
            <input style={num} value={maxRedirects} onChange={(e) => setMaxRedirects(e.target.value)} placeholder="5" disabled={connected || busy} />
          </div>
          <div style={row}>
            <label style={sub}>storage passphrase</label>
            <input style={input} type="password" value={storagePassphrase} onChange={(e) => setStoragePassphrase(e.target.value)} placeholder="(plaintext if empty)" disabled={connected || busy} />
          </div>
        </div>
      )}

      <LogPanel lines={lines('tunnel')} placeholder="Press Connect to bring up the tunnel." />
    </div>
  );
}
