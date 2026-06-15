import {
  didNetworkRefreshSucceed,
  resolveNetworkSwitchOutcome,
  selectNetworkForPersistence,
  shouldClearWalletUiStateOnNetworkSwitchCommit,
  shouldShowNetworkSwitchFailureToast,
} from './networkSwitchExecution';

describe('didNetworkRefreshSucceed', () => {
  it('treats a missing loaded client as a failed switch (loadAccount swallows errors)', () => {
    expect(didNetworkRefreshSucceed(undefined)).toBe(false);
    expect(didNetworkRefreshSucceed(null)).toBe(false);
  });

  it('treats a loaded client as a successful switch', () => {
    expect(didNetworkRefreshSucceed({ client_address: 'n1abc' })).toBe(true);
  });
});

describe('selectNetworkForPersistence', () => {
  it('prefers the committed network over stale React state when persisting after a switch', () => {
    expect(selectNetworkForPersistence('MAINNET', 'SANDBOX')).toBe('SANDBOX');
  });

  it('falls back to React state when no explicit network is provided', () => {
    expect(selectNetworkForPersistence('MAINNET')).toBe('MAINNET');
  });
});

describe('resolveNetworkSwitchOutcome', () => {
  it('keeps the previous network when refresh fails for a logged-in session', () => {
    expect(resolveNetworkSwitchOutcome('MAINNET', 'SANDBOX', false, true)).toStrictEqual({
      status: 'failed',
      network: 'MAINNET',
    });
  });

  it('commits the target network after refresh succeeds', () => {
    expect(resolveNetworkSwitchOutcome('MAINNET', 'SANDBOX', true, true)).toStrictEqual({
      status: 'committed',
      network: 'SANDBOX',
    });
  });
});

describe('shouldClearWalletUiStateOnNetworkSwitchCommit', () => {
  it('clears wallet UI state only after a committed switch', () => {
    expect(shouldClearWalletUiStateOnNetworkSwitchCommit({ status: 'failed', network: 'MAINNET' })).toBe(false);
    expect(shouldClearWalletUiStateOnNetworkSwitchCommit({ status: 'committed', network: 'SANDBOX' })).toBe(true);
  });
});

describe('shouldShowNetworkSwitchFailureToast', () => {
  it('does not duplicate loadAccount error toasts', () => {
    expect(shouldShowNetworkSwitchFailureToast()).toBe(false);
  });
});
