// Live mix-tunnel demo. Committed pre-publish and NOT yet imported by any page;
// see ./shared.tsx for why the import is dynamic and how to activate.
//
// mix-tunnel has no data operation of its own — it owns the shared tunnel the
// feature packages ride on — so this demo just brings the tunnel up, polls its
// state, and tears it down.

import React, { useRef, useState } from 'react';
import { DemoFrame, useLog, btnStyle, type MixTunnelModule } from './shared';

export function MixTunnelDemo() {
  const { lines, log } = useLog();
  const [up, setUp] = useState(false);
  const [busy, setBusy] = useState(false);
  const mod = useRef<MixTunnelModule | null>(null);

  async function connect() {
    setBusy(true);
    try {
      log('Loading mix-tunnel (wasm)...');
      // @ts-ignore -- @nymproject/mix-tunnel is published separately; absent at build time pre-publish
      mod.current = (await import('@nymproject/mix-tunnel')) as unknown as MixTunnelModule;

      log('setupMixTunnel()...');
      await mod.current.setupMixTunnel({ debug: true });

      const { state, reason } = await mod.current.getTunnelState();
      log(`state: ${state}${reason ? ` (${reason})` : ''}`);
      setUp(state === 'ready');
    } catch (err) {
      log(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  async function disconnect() {
    if (!mod.current) return;
    setBusy(true);
    try {
      log('disconnectMixTunnel()...');
      await mod.current.disconnectMixTunnel();
      const { state } = await mod.current.getTunnelState();
      log(`state: ${state}`);
      setUp(false);
    } catch (err) {
      log(`Error: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      setBusy(false);
    }
  }

  return (
    <DemoFrame title="setupMixTunnel" lines={lines}>
      <button style={btnStyle} onClick={connect} disabled={busy || up}>
        {busy && !up ? 'Connecting...' : 'Connect'}
      </button>
      <button style={btnStyle} onClick={disconnect} disabled={busy || !up}>
        Disconnect
      </button>
    </DemoFrame>
  );
}
