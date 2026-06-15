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

  it('blocks proceed when backup paths failed to load even if backup is acknowledged', () => {
    expect(
      canProceedToAccountRemovalPassword({
        backupConfirmed: true,
        pathsLoadError: 'IPC unavailable',
      }),
    ).toBe(false);
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
