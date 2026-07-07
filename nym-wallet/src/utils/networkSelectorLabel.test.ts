import { formatNetworkSelectorLabel } from './networkSelectorLabel';

const selectableNetworks = [
  { networkName: 'MAINNET' as const, name: 'Nym Mainnet' },
  { networkName: 'SANDBOX' as const, name: 'Testnet Sandbox' },
];

describe('formatNetworkSelectorLabel', () => {
  it('uses the friendly label for supported networks', () => {
    expect(formatNetworkSelectorLabel('MAINNET', selectableNetworks)).toBe('Nym Mainnet');
  });

  it('shows a readable legacy label when a persisted network is no longer selectable', () => {
    expect(formatNetworkSelectorLabel('QA', selectableNetworks)).toBe('QA (unsupported)');
  });

  it('falls back to the first selectable network when no network is set', () => {
    expect(formatNetworkSelectorLabel(undefined, selectableNetworks)).toBe('Nym Mainnet');
  });
});
