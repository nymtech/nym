import { shouldRefreshAccountOnManualNetworkSwitch } from './networkSwitchPolicy';

describe('networkSwitchPolicy', () => {
  it('requires backend refresh when user switches network while logged in', () => {
    expect(shouldRefreshAccountOnManualNetworkSwitch(true)).toBe(true);
  });

  it('defers account load to the network effect when there is no active session', () => {
    expect(shouldRefreshAccountOnManualNetworkSwitch(false)).toBe(false);
  });
});
