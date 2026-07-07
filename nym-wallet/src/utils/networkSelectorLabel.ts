import { Network } from '../types';

export type SelectableNetwork = { networkName: Network; name: string };

/** Display label for the header control; legacy persisted values stay readable after options are removed. */
export function formatNetworkSelectorLabel(
  network: string | undefined,
  selectableNetworks: readonly SelectableNetwork[],
): string {
  const match = selectableNetworks.find((entry) => entry.networkName === network);
  if (match) {
    return match.name;
  }
  if (network) {
    return `${network} (unsupported)`;
  }
  return selectableNetworks[0]?.name ?? 'Network';
}
