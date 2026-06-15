import { performAccountRemoval } from './accountRemovalFlow';

const account = { id: 'Account 2', address: 'addr2' };

describe('performAccountRemoval', () => {
  it('removes via IPC then reloads and returns the refreshed account list', async () => {
    const remaining = [{ id: 'Account 1', address: 'addr1' }];
    const removeAccount = jest.fn().mockResolvedValue(undefined);
    const reloadStoredAccounts = jest.fn().mockResolvedValue(remaining);

    const result = await performAccountRemoval({ account, password: 'pw', removeAccount, reloadStoredAccounts });

    expect(removeAccount).toHaveBeenCalledWith({ password: 'pw', accountName: 'Account 2' });
    expect(reloadStoredAccounts).toHaveBeenCalledTimes(1);
    expect(result).toStrictEqual(remaining);
  });

  it('does not reload the account list when the IPC removal fails', async () => {
    const removeAccount = jest.fn().mockRejectedValue('Switch to another account before removing the active account');
    const reloadStoredAccounts = jest.fn().mockResolvedValue([]);

    await expect(
      performAccountRemoval({ account, password: 'pw', removeAccount, reloadStoredAccounts }),
    ).rejects.toThrow('Switch to another account');
    expect(reloadStoredAccounts).not.toHaveBeenCalled();
  });

  it('maps a legacy mnemonic backend error to user-facing copy', async () => {
    const removeAccount = jest.fn().mockRejectedValue('Unexpected mnemonic account for login');
    const reloadStoredAccounts = jest.fn();

    await expect(
      performAccountRemoval({ account, password: 'pw', removeAccount, reloadStoredAccounts }),
    ).rejects.toThrow('legacy single-account');
  });
});
