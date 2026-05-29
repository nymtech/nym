// @nymproject/mix-dns
//
// Hostname-to-IP resolution over the Nym mixnet. Travels the IPR's DNS
// path (UDP) without setting up a TCP or TLS connection.

import {
  getMixTunnel,
  setupMixTunnel,
  disconnectMixTunnel,
  getTunnelState,
  SetupMixTunnelOpts,
} from '@nymproject/mix-tunnel';

export { setupMixTunnel, disconnectMixTunnel, getTunnelState };
export type { SetupMixTunnelOpts };

/**
 * Resolve a hostname through the mixnet. Returns the IP as a string
 * (e.g. `"93.184.216.34"`).
 *
 * The tunnel must already be set up via `setupMixTunnel()`.
 */
export const mixDNS = async (hostname: string): Promise<string> => {
  const tunnel = await getMixTunnel();
  return tunnel.mixDNS(hostname);
};
