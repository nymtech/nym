// Shared "Watch the Network tab" callout, used on the playground and the demo
// pages. Generic wording so it reads correctly wherever a single mixnet tunnel
// carries the page's traffic.
import React from 'react';
import { Callout } from 'nextra/components';

export function NetworkTabCallout() {
  return (
    <Callout type="info">
      <strong>Watch the Network tab.</strong> Open DevTools → Network before you connect. Once the
      tunnel reports ready, every operation you run here adds <strong>no new request</strong> to that
      tab: it is multiplexed inside the single WebSocket to the entry gateway. Only the clearnet
      comparison buttons add rows. (Setup also fetches the network topology over HTTPS and refreshes
      it periodically, so those nym-api calls and the gateway WebSocket are the only clearnet requests
      you will see.) Your real traffic never leaves the browser as an identifiable, per-destination
      request.
    </Callout>
  );
}
