import { didNetworkRefreshSucceed, resolveNetworkSwitchOutcome } from './networkSwitchExecution';

describe('didNetworkRefreshSucceed', () => {
  it('treats a missing loaded client as a failed switch (loadAccount swallows errors)', () => {
    expect(didNetworkRefreshSucceed(undefined)).toBe(false);
    expect(didNetworkRefreshSucceed(null)).toBe(false);
  });

  it('treats a loaded client as a successful switch', () => {
    expect(didNetworkRefreshSucceed({ client_address: 'n1abc' })).toBe(true);
  });
});

describe('resolveNetworkSwitchOutcome', () => {
  it('keeps the previous network when refresh fails for a logged-in session', () => {
    expect(
      resolveNetworkSwitchOutcome('MAINNET', 'SANDBOX', false, true),
    ).toEqual({ status: 'failed', network: 'MAINNET' });
  });

  it('commits the target network after refresh succeeds', () => {
    expect(
      resolveNetworkSwitchOutcome('MAINNET', 'SANDBOX', true, true),
    ).toEqual({ status: 'committed', network: 'SANDBOX' });
  });
});
