import { canRemoveAccount, getAccountRemovalBlockMessage, getAccountRemovalBlockReason } from './accountRemovalPolicy';

const accounts = [
  { id: 'Account 1', address: 'addr1' },
  { id: 'Account 2', address: 'addr2' },
];

describe('accountRemovalPolicy', () => {
  it('allows removing a non-active account when multiple accounts exist', () => {
    expect(canRemoveAccount(accounts, 'Account 1', 'Account 2')).toBe(true);
    expect(getAccountRemovalBlockReason(accounts, 'Account 1', 'Account 2')).toBeNull();
  });

  it('blocks removing the active account', () => {
    expect(canRemoveAccount(accounts, 'Account 1', 'Account 1')).toBe(false);
    expect(getAccountRemovalBlockReason(accounts, 'Account 1', 'Account 1')).toBe('active_account');
    expect(getAccountRemovalBlockMessage('active_account')).toContain('Switch to another account');
  });

  it('blocks removing the only stored account', () => {
    const single = [{ id: 'Account 1', address: 'addr1' }];
    expect(canRemoveAccount(single, 'Account 1', 'Account 1')).toBe(false);
    expect(getAccountRemovalBlockReason(single, 'Account 1', 'Account 1')).toBe('last_account');
    expect(getAccountRemovalBlockMessage('last_account')).toContain('only stored account');
  });
});
