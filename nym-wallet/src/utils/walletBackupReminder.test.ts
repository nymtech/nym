import { formatWalletBackupReminder } from './walletBackupReminder';

describe('walletBackupReminder', () => {
  it('includes platform paths and permanent removal warning', () => {
    const message = formatWalletBackupReminder({
      walletFile: '/Users/me/Library/Application Support/nym-wallet/saved-wallet.json',
      storageDirectory: '/Users/me/Library/Application Support/nym-wallet',
      configDirectory: '/Users/me/Library/Application Support/nym-wallet',
    });

    expect(message).toContain('permanently deletes');
    expect(message).toContain('cannot be undone');
    expect(message).toContain('saved-wallet.json');
    expect(message).toContain('backup');
    expect(message).toContain('Wallet folder');
  });
});
