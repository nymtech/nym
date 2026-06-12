import { mapAccountRemovalError } from './accountRemovalErrors';

describe('mapAccountRemovalError', () => {
  it('maps legacy mnemonic wallet errors', () => {
    expect(mapAccountRemovalError('Unexpected mnemonic account for login')).toContain('legacy single-account');
  });

  it('maps last-account errors', () => {
    expect(mapAccountRemovalError('Cannot remove the only stored account. Back up and remove the wallet file to reset entirely.')).toContain(
      'only stored account',
    );
  });

  it('maps active-account errors', () => {
    expect(
      mapAccountRemovalError('Switch to another account before removing the active account'),
    ).toContain('Switch to another account');
  });
});
