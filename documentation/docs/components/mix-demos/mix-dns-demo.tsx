// Live mix-dns demo. Committed pre-publish and NOT yet imported by any page;
// see ./shared.tsx for why the import is dynamic and how to activate.

import React, { useRef, useState } from 'react';
import {
  DemoFrame,
  useLog,
  btnStyle,
  inputStyle,
  type MixDnsModule,
} from './shared';

const DEFAULT_HOST = 'example.com';

export function MixDnsDemo() {
  const { lines, log } = useLog();
  const [host, setHost] = useState(DEFAULT_HOST);
  const [busy, setBusy] = useState(false);
  const mod = useRef<MixDnsModule | null>(null);

  async function run() {
    setBusy(true);
    try {
      if (!mod.current) {
        log('Loading mix-dns (wasm)...');
        // @ts-ignore -- @nymproject/mix-dns is published separately; absent at build time pre-publish
        mod.current = (await import('@nymproject/mix-dns')) as unknown as MixDnsModule;
        log('Bringing up the mixnet tunnel...');
        await mod.current.setupMixTunnel();
        log('Tunnel ready.');
      }

      log(`Resolving ${host} via the IPR...`);
      const ip = await mod.current.mixDNS(host);
      log(`< ${host} -> ${ip}`);
      log('(the resolver saw the query from the IPR, not from you)');
    } catch (err) {
      log(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <DemoFrame title="mixDNS" lines={lines}>
      <input
        style={inputStyle}
        value={host}
        onChange={(e) => setHost(e.target.value)}
        placeholder="example.com"
        disabled={busy}
      />
      <button style={btnStyle} onClick={run} disabled={busy}>
        {busy ? 'Resolving...' : 'Resolve'}
      </button>
    </DemoFrame>
  );
}
