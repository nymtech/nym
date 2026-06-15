// ENS-over-the-mixnet demo, ported from wasm/ens-demo. Resolve <name>.eth to an
// address + contenthash, then fetch the IPFS site, every byte through mixFetch.
// The tunnel lifecycle + options live in <MixTunnelSetup>; this component owns
// the ENS flow and receives a `mixFetch` when the tunnel is ready.

import React, { useRef, useState } from 'react';
import type { JsonRpcProvider } from 'ethers';
import { MixTunnelSetup, type MixFetchFn } from '../shared/mixTunnel';
import { Button, LogPanel, useLogs, box, row, input, sub, legend } from '../shared/ui';
import { buildProvider, callMixFetch, decompressBody, expandGatewayUrl, formatSize, htmlFingerprint, renderFingerprint } from './lib';

const NAME_PRESETS = ['vitalik.eth', 'ens.eth', 'gregskril.eth', 'raffy.eth', 'luc.eth'];
const RPC_PRESETS = ['https://ethereum-rpc.publicnode.com', 'https://rpc.ankr.com/eth', 'https://eth.public-rpc.com'];
const GATEWAY_PRESETS = ['https://{cid}.ipfs.dweb.link/', 'https://dweb.link/ipfs/{cid}/'];

const IP_ECHO_URL = 'https://ipinfo.io/ip';
// Rough IPv4/IPv6 shape check, to catch an HTML or error body returned in place
// of an IP address. Not a full validator (it does not range-check octets).
const IP_SHAPE_RE = /^(?:\d{1,3}(?:\.\d{1,3}){3}|[0-9a-f]{0,4}(?::[0-9a-f]{0,4}){2,7})$/i;

const preStyle: React.CSSProperties = {
  maxHeight: 240,
  overflowY: 'auto',
  fontSize: 12.5,
  whiteSpace: 'pre-wrap',
  overflowWrap: 'anywhere',
  background: 'rgba(127,127,127,0.06)',
  border: '1px solid rgba(127,127,127,0.2)',
  borderRadius: 6,
  padding: '0.5rem',
  margin: '0.5rem 0 0',
};

export function EnsDemo() {
  const { log, lines } = useLogs();
  const [mixFetch, setMixFetch] = useState<MixFetchFn | null>(null);
  const providerRef = useRef<JsonRpcProvider | null>(null);

  const [ensName, setEnsName] = useState('vitalik.eth');
  const [ensRpc, setEnsRpc] = useState(RPC_PRESETS[0]);
  const [gateway, setGateway] = useState(GATEWAY_PRESETS[0]);
  const [customCid, setCustomCid] = useState('');
  const [lastCid, setLastCid] = useState<string | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [verifyLink, setVerifyLink] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const connected = mixFetch != null;
  const ensLog = (msg: string, colour?: 'green' | 'red' | 'orange' | 'gray') => log('ens', msg, colour);

  function ensureProvider(): JsonRpcProvider {
    if (providerRef.current) return providerRef.current;
    const rpc = ensRpc.trim();
    if (!rpc) throw new Error('RPC URL is required');
    ensLog(`building JsonRpcProvider({ rpc: ${rpc}, transport: mixFetch })`);
    providerRef.current = buildProvider(rpc, mixFetch!, ensLog);
    return providerRef.current;
  }

  function onReady(fn: MixFetchFn) {
    setMixFetch(() => fn);
  }
  function onDisconnect() {
    setMixFetch(null);
    providerRef.current = null;
    setLastCid(null);
  }

  async function resolveAddress() {
    const name = ensName.trim();
    if (!name) return ensLog('Name is required', 'red');
    let provider: JsonRpcProvider;
    try {
      provider = ensureProvider();
    } catch (e) {
      return ensLog(`${e}`, 'red');
    }
    ensLog(`step 1/3: resolving ${name} via ENS Registry + Resolver`);
    const t0 = performance.now();
    try {
      const addr = await provider.resolveName(name);
      const ms = (performance.now() - t0).toFixed(0);
      if (addr) ensLog(`${name} -> ${addr} (${ms} ms total)`, 'green');
      else ensLog(`${name} has no addr record (${ms} ms)`, 'orange');
    } catch (e: any) {
      ensLog(`resolveName failed: ${e.shortMessage || e.message || e}`, 'red');
    }
  }

  async function getContenthash() {
    const name = ensName.trim();
    if (!name) return ensLog('Name is required', 'red');
    let provider: JsonRpcProvider;
    try {
      provider = ensureProvider();
    } catch (e) {
      return ensLog(`${e}`, 'red');
    }
    ensLog(`step 2/3: reading contenthash record from ${name}'s resolver`);
    const t0 = performance.now();
    try {
      const resolver = await provider.getResolver(name);
      if (!resolver) return ensLog(`${name} has no resolver`, 'orange');
      const content = await resolver.getContentHash();
      const ms = (performance.now() - t0).toFixed(0);
      if (!content) return ensLog(`${name} has no contenthash (${ms} ms)`, 'orange');
      ensLog(`contenthash: ${content} (${ms} ms total)`, 'green');
      const ipfsMatch = content.match(/^ipfs:\/\/(.+)$/);
      if (ipfsMatch) {
        setLastCid(ipfsMatch[1]);
        ensLog(`decoded CID: ${ipfsMatch[1]}`);
      } else {
        ensLog('non-IPFS scheme in contenthash; nothing to fetch', 'orange');
      }
    } catch (e: any) {
      ensLog(`contenthash lookup failed: ${e.shortMessage || e.message || e}`, 'red');
    }
  }

  async function fetchIpfsCid(cid: string, label: string) {
    if (!cid) return ensLog(`${label}: CID is required`, 'red');
    const gw = gateway.trim();
    if (!gw) return ensLog('IPFS gateway is required', 'red');
    const url = expandGatewayUrl(gw, cid);
    ensLog(`${label} GET ${url}`);
    const t0 = performance.now();
    try {
      const raw = await callMixFetch(mixFetch!, url, {});
      const buf = await decompressBody(raw.body, raw.headers);
      const ms = (performance.now() - t0).toFixed(0);
      const ctype = raw.headers['content-type'] || '';
      const wireSize = raw.body ? raw.body.byteLength : 0;
      const wireNote = wireSize !== buf.byteLength ? ` (${formatSize(wireSize)} wire, decompressed)` : '';
      ensLog(`${raw.status} ${raw.statusText}: ${formatSize(buf.byteLength)} ${ctype}${wireNote} in ${ms} ms`, 'green');

      const text = new TextDecoder('utf-8', { fatal: false }).decode(buf);
      const looksLikeHtml = ctype.includes('html') || /<html\b|<!doctype html/i.test(text.slice(0, 1000));
      if (looksLikeHtml) {
        const fp = htmlFingerprint(text);
        if (fp.title) ensLog(`page title: "${fp.title}"`, 'green');
        setPreview(renderFingerprint(fp, buf.byteLength));
      } else if (ctype.includes('json')) {
        try {
          setPreview(JSON.stringify(JSON.parse(text), null, 2));
        } catch {
          setPreview(text);
        }
      } else if (ctype.includes('text/')) {
        setPreview(text);
      } else {
        setPreview(`[binary content, ${formatSize(buf.byteLength)}, ${ctype || 'unknown type'}]`);
      }
      setVerifyLink(`https://ipfs.io/ipfs/${cid}/`);
      ensLog('Visual content check (open the link below in another tab), not CID-hash verification.', 'gray');
    } catch (e: any) {
      ensLog(`${label} fetch failed: ${e.message || e}`, 'red');
      ensLog("If this is a 403/429 or connection error, the exit IP may be rate-limited. Tick 'Use random IPR' and reload.", 'orange');
    }
  }

  async function verifyIp() {
    if (!mixFetch) return ensLog('connect the mixnet tunnel first', 'red');
    setBusy(true);
    ensLog('comparing direct-clearnet IP vs Nym-exit IP...');
    let directIp: string;
    try {
      const text = (await (await fetch(IP_ECHO_URL)).text()).trim();
      directIp = IP_SHAPE_RE.test(text) ? text : `(unexpected: ${text})`;
    } catch (e: any) {
      directIp = `error: ${e.message || e}`;
    }
    ensLog(`  your real IP (direct fetch, no Nym): ${directIp}`, 'orange');
    let nymIp: string;
    try {
      const raw = await callMixFetch(mixFetch, IP_ECHO_URL, {});
      const text = new TextDecoder().decode(raw.body).trim();
      nymIp = IP_SHAPE_RE.test(text) ? text : `(unexpected: ${text})`;
    } catch (e: any) {
      nymIp = `error: ${e.message || e}`;
    }
    ensLog(`  what the upstream sees via mixFetch -> Nym: ${nymIp}`, 'green');
    if (!nymIp.startsWith('error') && !directIp.startsWith('error') && nymIp !== directIp) {
      ensLog('IPs differ. The RPC and gateway see the Nym exit, not you. Every ENS step uses the same path.', 'green');
    } else if (nymIp.startsWith('error') || directIp.startsWith('error')) {
      ensLog('Could not complete the comparison. Try again, or reconnect with a different IPR.', 'red');
    } else {
      ensLog('IPs match. The mixnet route may not be active, or the IP service is behind a shared CDN. Try again.', 'red');
    }
    setBusy(false);
  }

  return (
    <div style={{ margin: '1.5rem 0' }}>
      <MixTunnelSetup onReady={onReady} onDisconnect={onDisconnect} clientIdPrefix="ens-demo" />

      <div style={box}>
        <div style={legend}>ENS lookup</div>
        <div style={row}>
          <Button onClick={verifyIp} disabled={!connected || busy}>Verify IP routing</Button>
          <span style={sub}>Confirms traffic exits through Nym. The comparison makes one direct (clearnet) call to ipinfo.io, so you will see a single ipinfo.io row in the Network tab.</span>
        </div>

        <div style={row}>
          <label style={sub}>Name</label>
          <select style={input} value={ensName} onChange={(e) => setEnsName(e.target.value)} disabled={!connected}>
            {NAME_PRESETS.map((n) => <option key={n} value={n}>{n}</option>)}
          </select>
          <input style={input} value={ensName} onChange={(e) => setEnsName(e.target.value)} disabled={!connected} />
        </div>
        <div style={row}>
          <label style={sub}>RPC</label>
          <select style={input} value={RPC_PRESETS.includes(ensRpc) ? ensRpc : ''} onChange={(e) => { setEnsRpc(e.target.value); providerRef.current = null; }} disabled={!connected}>
            {RPC_PRESETS.map((u) => <option key={u} value={u}>{u}</option>)}
          </select>
          <input style={input} value={ensRpc} onChange={(e) => { setEnsRpc(e.target.value); providerRef.current = null; }} disabled={!connected} />
        </div>
        <div style={row}>
          <label style={sub}>IPFS gateway</label>
          <select style={input} value={GATEWAY_PRESETS.includes(gateway) ? gateway : ''} onChange={(e) => setGateway(e.target.value)} disabled={!connected}>
            {GATEWAY_PRESETS.map((g) => <option key={g} value={g}>{g}</option>)}
          </select>
          <input style={input} value={gateway} onChange={(e) => setGateway(e.target.value)} disabled={!connected} />
        </div>

        <div style={row}>
          <Button onClick={resolveAddress} disabled={!connected}>1. Resolve address</Button>
          <Button onClick={getContenthash} disabled={!connected}>2. Get contenthash</Button>
          <Button onClick={() => lastCid && fetchIpfsCid(lastCid, 'step 3/3:')} disabled={!connected || !lastCid}>3. Fetch from IPFS</Button>
        </div>
        <div style={row}>
          <label style={sub}>Or fetch any CID</label>
          <input style={input} value={customCid} onChange={(e) => setCustomCid(e.target.value)} placeholder="bafybe... or Qm..." disabled={!connected} />
          <Button onClick={() => fetchIpfsCid(customCid.trim(), 'custom')} disabled={!connected || !customCid.trim()}>Fetch</Button>
        </div>

        <LogPanel lines={lines('ens')} placeholder="Connect the tunnel, then run a lookup." />
        {verifyLink && (
          <div style={{ ...sub, marginTop: '0.4rem' }}>
            verify visually in another tab: <a href={verifyLink} target="_blank" rel="noopener noreferrer">{verifyLink}</a>
          </div>
        )}
        {preview != null && <pre style={preStyle}>{preview}</pre>}
      </div>
    </div>
  );
}
