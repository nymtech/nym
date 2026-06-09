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
        <strong>Mixnet.</strong> An overlay network that routes your traffic through several relays
        and mixes it with other people's, hiding who is talking to whom. Nym operates one. See{' '}
        <L href="/network/mixnet-mode">mixnet mode</L>.
      </li>
      <li>
        <strong>Entry gateway.</strong> Your first hop into the mixnet. Your browser holds one
        WebSocket to it; all tunnelled traffic rides that connection as opaque frames. See{' '}
        <L href="/network/infrastructure/nym-nodes">Nym nodes</L>.
      </li>
      <li>
        <strong>IPR (IP Packet Router), the exit.</strong> The mixnet's exit point onto the normal
        internet. The RPC node and gateway see the IPR's IP address, never yours. See{' '}
        <L href="/network/infrastructure/exit-services#ip-packet-router">exit services</L>.
      </li>
      <li>
        <strong>SURB (single-use reply block).</strong> A prepaid, single-use return envelope. It
        lets the exit send a reply back through the mixnet without learning your address. See{' '}
        <L href="/network/mixnet-mode/anonymous-replies">anonymous replies</L>.
      </li>
      <li>
        <strong>Cover traffic / Poisson timing.</strong> Decoy packets and randomised send timing.
        Together they keep your real traffic statistically hard to pick out. See{' '}
        <L href="/network/mixnet-mode/cover-traffic">cover traffic</L>.
      </li>
      <li>
        <strong>mixFetch.</strong> The{' '}
        <L href="/developers/mix-fetch"><code>@nymproject/mix-fetch</code></L> package's{' '}
        <code>fetch()</code>-shaped function. It runs the mixnet client (smolmix) in a Web Worker and
        sends your request through the mixnet instead of the browser's network stack.
      </li>
    </ul>
  );
}
