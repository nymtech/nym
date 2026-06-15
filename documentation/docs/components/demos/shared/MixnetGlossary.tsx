// Shared mixnet glossary for the demo pages, with links to the relevant docs.
// Raw <a> inside a React component does not pick up Nextra's MDX link styling,
// so the links are styled explicitly via the L helper.
import React from 'react';

function L({ href, children }: { href: string; children: React.ReactNode }) {
  return (
    <a href={href} style={{ color: '#3b82f6', textDecoration: 'underline' }}>
      {children}
    </a>
  );
}

export function MixnetGlossary() {
  return (
    <ul>
      <li>
        <strong>Mixnet.</strong> An overlay network that routes your traffic through several relays,
        mixed in with everyone else's, so no single point can link sender to receiver. See{' '}
        <L href="/network/mixnet-mode">mixnet mode</L>.
      </li>
      <li>
        <strong>Entry gateway.</strong> Your first hop into the mixnet. The browser holds one
        WebSocket to it, and all tunnelled traffic travels over that single connection as opaque
        frames. See <L href="/network/infrastructure/nym-nodes">Nym nodes</L>.
      </li>
      <li>
        <strong>IPR (IP Packet Router), the exit.</strong> Where traffic leaves the mixnet for the
        public internet. The destination sees the IPR's IP, not yours. See{' '}
        <L href="/network/infrastructure/exit-services#ip-packet-router">exit services</L>.
      </li>
      <li>
        <strong>SURB (single-use reply block).</strong> A prepaid, single-use return envelope. The
        exit replies through it without ever learning your address. See{' '}
        <L href="/network/mixnet-mode/anonymous-replies">anonymous replies</L>.
      </li>
      <li>
        <strong>Cover traffic / Poisson timing.</strong> Decoy packets sent on randomised timing, so
        your real traffic blends into a steady stream. See{' '}
        <L href="/network/mixnet-mode/cover-traffic">cover traffic</L>.
      </li>
      <li>
        <strong>mixFetch.</strong> A <code>fetch()</code>-shaped function from{' '}
        <L href="/developers/mix-fetch"><code>@nymproject/mix-fetch</code></L>. It runs the mixnet
        client (smolmix) in a Web Worker, so each request goes through the mixnet rather than the
        browser's network stack.
      </li>
    </ul>
  );
}
