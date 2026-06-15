// Railgun-over-the-mixnet demo, ported from wasm/railgun-demo. Two privacy
// layers: Nym hides the network (RPC via mixFetch), Railgun hides the
// application layer (shielded notes). Sepolia testnet only.

import React, { useEffect, useRef, useState } from 'react';
import { HDNodeWallet, JsonRpcProvider, formatEther } from 'ethers';
import { MixTunnelSetup, type MixFetchFn } from '../shared/mixTunnel';
import { Button, LogPanel, Spinner, StatusText, useLogs, box, row, input, sub, legend } from '../shared/ui';
import { buildProvider, callMixFetch, installGlobalMixFetchRouting, withRetry } from '../shared/mixfetch';
import {
  DEFAULT_MNEMONIC,
  SEPOLIA_CHAIN_ID,
  STORAGE_KEY,
  createRailgunWalletFromMnemonic,
  derivePublicAddress,
  ensureRailgunEngine,
  shieldEth,
  type RailgunWalletInfo,
} from './lib';

const RPC_PRESETS = ['https://ethereum-sepolia-rpc.publicnode.com', 'https://rpc.sepolia.org'];
// Fixed shield amount: a single small value so the shared, faucet-funded testnet
// wallet can't be drained by an arbitrary amount.
const SHIELD_AMOUNT = '0.01';
const IP_ECHO_URL = 'https://ipinfo.io/ip';
// Rough IPv4/IPv6 shape check, to catch an HTML or error body returned in place
// of an IP address. Not a full validator (it does not range-check octets).
const IP_SHAPE_RE = /^(?:\d{1,3}(?:\.\d{1,3}){3}|[0-9a-f]{0,4}(?::[0-9a-f]{0,4}){2,7})$/i;

export function RailgunDemo() {
  const { log, lines } = useLogs();
  const dlog = (msg: string, colour?: 'green' | 'red' | 'orange' | 'gray') => log('railgun', msg, colour);

  const [mixFetch, setMixFetch] = useState<MixFetchFn | null>(null);
  const [mnemonic, setMnemonic] = useState('');
  const [publicAddr, setPublicAddr] = useState('(not generated)');
  const [railgunWallet, setRailgunWallet] = useState<RailgunWalletInfo | null>(null);
  // Engine init is the slow, cold-route step. Surface it as its own status line
  // under the tunnel; the detailed log lines still flow to the shield console.
  const [enginePhase, setEnginePhase] = useState<'idle' | 'initialising' | 'ready' | 'error'>('idle');
  const [engineError, setEngineError] = useState('');
  const [rpc, setRpc] = useState(RPC_PRESETS[0]);
  const [balance, setBalance] = useState('');
  const [txHash, setTxHash] = useState<string | null>(null);
  const [storageStatus, setStorageStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const [importPhrase, setImportPhrase] = useState('');

  const publicWalletRef = useRef<HDNodeWallet | null>(null);
  const providerRef = useRef<JsonRpcProvider | null>(null);

  const connected = mixFetch != null;
  const hasWallet = publicAddr !== '(not generated)';

  function updateStorageStatus() {
    let stored: string | null = null;
    try {
      stored = localStorage.getItem(STORAGE_KEY);
    } catch {
      /* localStorage disabled */
    }
    setStorageStatus(stored ? 'wallet saved in browser storage (auto-loaded on reload)' : 'no wallet saved; generate or import to persist one');
  }

  function ensureProvider(): JsonRpcProvider {
    if (providerRef.current) return providerRef.current;
    if (!mixFetch) throw new Error('connect the mixnet tunnel first');
    const url = rpc.trim();
    if (!url) throw new Error('Sepolia RPC URL is required');
    dlog(`building JsonRpcProvider({ rpc: ${url}, transport: mixFetch })`);
    providerRef.current = buildProvider(url, mixFetch, SEPOLIA_CHAIN_ID);
    return providerRef.current;
  }

  async function deriveRailgun(phrase: string) {
    setEnginePhase('initialising');
    setEngineError('');
    dlog('initialising Railgun engine + deriving shielded address...');
    try {
      await ensureRailgunEngine(rpc.trim(), dlog);
      const result = await createRailgunWalletFromMnemonic(phrase.trim());
      setRailgunWallet(result);
      setEnginePhase('ready');
      dlog(`Railgun address derived: ${result.railgunAddress}`, 'green');
    } catch (e: any) {
      setEnginePhase('error');
      setEngineError(e.message || String(e));
      dlog(`Railgun derivation failed: ${e.message || e}`, 'red');
    }
  }

  function loadWallet(phrase: string) {
    let wallet: HDNodeWallet;
    try {
      wallet = derivePublicAddress(phrase);
    } catch (e: any) {
      dlog(`invalid mnemonic: ${e.message || e}`, 'red');
      return;
    }
    publicWalletRef.current = wallet;
    setPublicAddr(wallet.address);
    setRailgunWallet(null);
    setEnginePhase('idle');
    setEngineError('');
    setMnemonic(phrase.trim());
    try {
      localStorage.setItem(STORAGE_KEY, phrase.trim());
    } catch {
      /* localStorage disabled */
    }
    updateStorageStatus();
    dlog(`public address derived: ${wallet.address}`, 'green');
    if (mixFetch) void deriveRailgun(phrase);
    else dlog('connect the mixnet tunnel to derive the Railgun address', 'orange');
  }

  // Auto-load on mount: stored mnemonic, else the funded testnet fallback.
  useEffect(() => {
    let stored: string | null = null;
    try {
      stored = localStorage.getItem(STORAGE_KEY);
    } catch {
      /* localStorage disabled */
    }
    const phrase = stored || DEFAULT_MNEMONIC;
    setMnemonic(phrase);
    updateStorageStatus();
    try {
      const wallet = derivePublicAddress(phrase);
      publicWalletRef.current = wallet;
      setPublicAddr(wallet.address);
      dlog(`auto-loaded wallet: ${wallet.address}`, 'green');
      dlog('public side ready. The Railgun address derives once the tunnel is up.');
    } catch (e: any) {
      dlog(`auto-load failed: ${e.message || e}`, 'red');
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function onReady(fn: MixFetchFn) {
    setMixFetch(() => fn);
    // Route every ethers HTTP call (incl. Railgun's internal providers) through Nym.
    installGlobalMixFetchRouting(fn);
    if (publicWalletRef.current && !railgunWallet) void deriveRailgun(mnemonic);
  }
  function onDisconnect() {
    setMixFetch(null);
    providerRef.current = null;
  }

  function generateWallet() {
    dlog('generating fresh BIP-39 mnemonic...');
    const wallet = HDNodeWallet.createRandom();
    loadWallet(wallet.mnemonic!.phrase);
  }
  function importWallet() {
    loadWallet(importPhrase);
    setImportPhrase('');
  }
  function clearWallet() {
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* localStorage disabled */
    }
    publicWalletRef.current = null;
    setPublicAddr('(not generated)');
    setRailgunWallet(null);
    setEnginePhase('idle');
    setEngineError('');
    setMnemonic('');
    updateStorageStatus();
    dlog('cleared stored wallet; reload to load the funded fallback');
  }

  async function checkBalance() {
    if (!publicWalletRef.current) return dlog('generate or import a wallet first', 'red');
    let provider: JsonRpcProvider;
    try {
      provider = ensureProvider();
    } catch (e: any) {
      return dlog(`${e.message || e}`, 'red');
    }
    setBusy(true);
    dlog(`eth_getBalance(${publicWalletRef.current.address}) via mixFetch...`);
    try {
      const wei = await withRetry(() => provider.getBalance(publicWalletRef.current!.address), 'eth_getBalance', { log: dlog });
      const eth = formatEther(wei);
      setBalance(`${eth} ETH (Sepolia)`);
      dlog(`balance: ${eth} ETH`, 'green');
    } catch (e: any) {
      dlog(`balance lookup failed: ${e.shortMessage || e.message || e}`, 'red');
    } finally {
      setBusy(false);
    }
  }

  async function shield() {
    if (!railgunWallet) return dlog('Railgun wallet not derived; connect tunnel + generate wallet first', 'red');
    if (!publicWalletRef.current) return dlog('public wallet missing', 'red');
    let provider: JsonRpcProvider;
    try {
      provider = ensureProvider();
    } catch (e: any) {
      return dlog(`${e.message || e}`, 'red');
    }
    setBusy(true);
    setTxHash(null);
    try {
      await shieldEth({
        publicWallet: publicWalletRef.current,
        railgunWallet,
        provider,
        amountStr: SHIELD_AMOUNT,
        log: dlog,
        onTxHash: setTxHash,
      });
    } catch (e: any) {
      dlog(`shield failed: ${e.shortMessage || e.message || e}`, 'red');
      // Tear down the provider so its background pollers stop after a failure.
      try {
        providerRef.current?.destroy();
      } catch {
        /* ignore */
      }
      providerRef.current = null;
    } finally {
      setBusy(false);
    }
  }

  async function verifyIp() {
    if (!mixFetch) return dlog('connect the mixnet tunnel first', 'red');
    setBusy(true);
    dlog('comparing direct-clearnet IP vs Nym-exit IP...');
    let directIp: string;
    try {
      directIp = (await (await fetch(IP_ECHO_URL)).text()).trim();
      if (!IP_SHAPE_RE.test(directIp)) directIp = `(unexpected: ${directIp})`;
    } catch (e: any) {
      directIp = `error: ${e.message || e}`;
    }
    dlog(`  your real IP (direct fetch, no Nym): ${directIp}`, 'orange');
    let nymIp: string;
    try {
      const raw = await callMixFetch(mixFetch, IP_ECHO_URL, {});
      nymIp = new TextDecoder().decode(raw.body).trim();
      if (!IP_SHAPE_RE.test(nymIp)) nymIp = `(unexpected: ${nymIp})`;
    } catch (e: any) {
      nymIp = `error: ${e.message || e}`;
    }
    dlog(`  what the upstream sees via mixFetch -> Nym: ${nymIp}`, 'green');
    if (!nymIp.startsWith('error') && !directIp.startsWith('error') && nymIp !== directIp) {
      dlog('IPs differ. Every Shield broadcast uses this same Nym-exit path.', 'green');
    } else if (nymIp.startsWith('error') || directIp.startsWith('error')) {
      dlog('Could not complete the comparison. Try again, or reconnect with a different IPR.', 'red');
    } else {
      dlog('IPs match: your traffic did NOT route through Nym. The route may not be active, or ipinfo.io is behind a shared CDN. Try again.', 'red');
    }
    setBusy(false);
  }

  return (
    <div style={{ margin: '1.5rem 0' }}>
      <div style={{ ...box, borderColor: 'var(--colorWarn, #d97706)' }}>
        <strong>Sepolia testnet only.</strong>{' '}
        <span style={sub}>
          The wallet holds only test ETH from public faucets and the mnemonic is stored in plain
          browser storage. Never paste a mainnet mnemonic into this demo.
        </span>
      </div>

      <MixTunnelSetup onReady={onReady} onDisconnect={onDisconnect} clientIdPrefix="railgun-demo" />

      <div style={box}>
        <div style={legend}>Railgun engine</div>
        <div style={sub}>
          The first init makes a cold-route call to Sepolia over the mixnet and can take
          10 seconds or more, sometimes after one internal retry. Detailed progress appears
          in the Shield console below.
        </div>
        <div style={{ ...row, marginTop: '0.6rem' }}>
          {enginePhase === 'initialising' ? (
            <Spinner label="initialising engine + deriving shielded address..." />
          ) : enginePhase === 'ready' ? (
            <StatusText status={{ text: 'Engine ready', colour: 'green' }} />
          ) : enginePhase === 'error' ? (
            <StatusText status={{ text: `Engine init failed: ${engineError}`, colour: 'red' }} />
          ) : (
            <StatusText
              status={{
                text: connected ? 'Generate or import a wallet to derive the shielded address.' : 'Connect the tunnel to start the engine.',
                colour: 'gray',
              }}
            />
          )}
        </div>
        <div style={sub}>Railgun address: <code>{railgunWallet ? railgunWallet.railgunAddress : connected ? '(deriving...)' : '(connect tunnel to derive)'}</code></div>
      </div>

      <div style={box}>
        <div style={legend}>Wallet</div>
        <div style={sub}>
          A testnet wallet is auto-loaded from browser storage. The fallback mnemonic lives in
          the demo source, so treat it as shared and public: it holds only Sepolia test ETH.
          Import your own below if you'd rather keep a separate balance.
        </div>
        <div style={row}>
          <input
            type="password"
            autoComplete="off"
            style={input}
            value={importPhrase}
            onChange={(e) => setImportPhrase(e.target.value)}
            placeholder="import your own 12-word mnemonic (optional)"
          />
          <Button onClick={importWallet} disabled={!importPhrase.trim()}>Import</Button>
          <Button onClick={generateWallet}>Generate</Button>
          <Button onClick={clearWallet}>Clear</Button>
        </div>
        <div style={sub}>public address: <code>{publicAddr}</code></div>
        <div style={sub}>{storageStatus}</div>
      </div>

      <div style={box}>
        <div style={legend}>Public Sepolia state</div>
        <div style={row}>
          <label style={sub}>RPC</label>
          <select style={input} value={RPC_PRESETS.includes(rpc) ? rpc : ''} disabled={connected || busy} onChange={(e) => { setRpc(e.target.value); providerRef.current = null; }}>
            {RPC_PRESETS.map((u) => <option key={u} value={u}>{u}</option>)}
          </select>
          <input style={input} value={rpc} disabled={connected || busy} onChange={(e) => { setRpc(e.target.value); providerRef.current = null; }} />
        </div>
        <div style={row}>
          <Button onClick={checkBalance} disabled={!connected || !hasWallet || busy}>Check balance</Button>
          <Button onClick={verifyIp} disabled={!connected || busy}>Verify IP routing</Button>
          <span style={sub}>{balance}</span>
        </div>
        <div style={sub}>Verify IP makes one direct (clearnet) call to ipinfo.io for the comparison, so you will see a single ipinfo.io row in the Network tab.</div>
      </div>

      <div style={box}>
        <div style={legend}>Shield ETH into a private note</div>
        <div style={row}>
          <Button onClick={shield} disabled={!connected || !railgunWallet || busy}>Shield {SHIELD_AMOUNT} ETH</Button>
          <span style={sub}>Fixed at {SHIELD_AMOUNT} ETH so the shared testnet wallet isn't drained.</span>
        </div>
        {txHash && (
          <div style={{ marginTop: '0.5rem' }}>
            <a
              href={`https://sepolia.etherscan.io/tx/${txHash}`}
              target="_blank"
              rel="noopener noreferrer"
              style={{ color: '#3b82f6', textDecoration: 'underline', fontWeight: 600 }}
            >
              View transaction on Etherscan
            </a>{' '}
            <code style={{ ...sub, opacity: 0.7 }}>{txHash.slice(0, 10)}...{txHash.slice(-8)}</code>
          </div>
        )}
        <LogPanel lines={lines('railgun')} placeholder="Connect the tunnel, then derive the wallet and shield." />
      </div>
    </div>
  );
}
