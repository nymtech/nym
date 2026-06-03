// Live mix-fetch demo. Committed pre-publish and NOT yet imported by any page;
// see ./shared.tsx for why the import is dynamic and how to activate.

import React, { useRef, useState } from 'react';
import {
  DemoFrame,
  useLog,
  btnStyle,
  inputStyle,
  type MixFetchModule,
} from './shared';

// Cloudflare's trace endpoint echoes the requesting IP back as `ip=...`, so the
// demo can show that the request exits at the IPR's address, not the browser's.
const DEFAULT_URL = 'https://www.cloudflare.com/cdn-cgi/trace';

export function MixFetchDemo() {
  const { lines, log } = useLog();
  const [url, setUrl] = useState(DEFAULT_URL);
  const [busy, setBusy] = useState(false);
  const mod = useRef<MixFetchModule | null>(null);

  async function run() {
    setBusy(true);
    try {
      if (!mod.current) {
        log('Loading mix-fetch (wasm)...');
        // @ts-ignore -- @nymproject/mix-fetch is published separately; absent at build time pre-publish
        mod.current = (await import('@nymproject/mix-fetch')) as unknown as MixFetchModule;
        log('Bringing up the mixnet tunnel...');
        await mod.current.setupMixTunnel();
        log('Tunnel ready.');
      }

      log(`GET ${url}`);
      const res = await mod.current.mixFetch(url);
      const body = await res.text();
      log(`< ${res.status} ${res.statusText}`);

      const ip = body.match(/(?:^|\n)ip=([^\n]+)/)?.[1];
      if (ip) log(`< exit IP (the IPR's, not yours): ${ip}`);
      log(body.length > 400 ? `${body.slice(0, 400)}\n... (${body.length} bytes total)` : body);
    } catch (err) {
      log(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <DemoFrame title="mixFetch" lines={lines}>
      <input
        style={inputStyle}
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="https://..."
        disabled={busy}
      />
      <button style={btnStyle} onClick={run} disabled={busy}>
        {busy ? 'Running...' : 'Run'}
      </button>
    </DemoFrame>
  );
}
