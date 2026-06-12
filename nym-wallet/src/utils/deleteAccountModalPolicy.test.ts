import { canProceedToAccountRemovalPassword } from './deleteAccountModalPolicy';

describe('canProceedToAccountRemovalPassword', () => {
  it('requires backup acknowledgement and loaded paths', () => {
    expect(
      canProceedToAccountRemovalPassword({
        backupConfirmed: true,
        backupReminder: '~/saved-wallet.json',
      }),
    ).toBe(true);
  });

  it('allows proceed when paths failed to load but backup is acknowledged', () => {
    expect(
      canProceedToAccountRemovalPassword({
        backupConfirmed: true,
        pathsLoadError: 'IPC unavailable',
      }),
    ).toBe(true);
  });

  it('blocks proceed without backup acknowledgement even if paths errored', () => {
    expect(
      canProceedToAccountRemovalPassword({
        backupConfirmed: false,
        pathsLoadError: 'IPC unavailable',
      }),
    ).toBe(false);
  });
});
