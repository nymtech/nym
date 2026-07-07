import { FamilyError } from 'src/types/families';
import { formatDurationSecs, formatExpiry, inviteWarningFromError } from './helpers';

// FamilyError is a real Error subclass; isFamilyError uses `instanceof`.

describe('formatExpiry', () => {
  it('returns Expired when the deadline has passed', () => {
    expect(formatExpiry(100, 100)).toBe('Expired');
    expect(formatExpiry(100, 200)).toBe('Expired');
  });

  it('formats sub-minute, minute, hour and day remainders', () => {
    expect(formatExpiry(130, 100)).toBe('in 30s');
    expect(formatExpiry(100 + 9 * 60 + 41, 100)).toBe('in 9m 41s');
    expect(formatExpiry(100 + 3 * 3600, 100)).toBe('in 3h');
    expect(formatExpiry(100 + 7 * 86400, 100)).toBe('in 7d');
  });
});

describe('formatDurationSecs', () => {
  it('renders config-driven TTLs', () => {
    expect(formatDurationSecs(600)).toBe('10 minutes');
    expect(formatDurationSecs(604800)).toBe('7 days');
    expect(formatDurationSecs(1)).toBe('1 second');
  });
});

describe('inviteWarningFromError', () => {
  it('maps typed FamilyError kinds', () => {
    expect(inviteWarningFromError(new FamilyError('NodeAlreadyInFamily'))).toBe('already-in-family');
    expect(inviteWarningFromError(new FamilyError('AlreadyInFamily'))).toBe('already-in-family');
  });

  it('maps a raw CosmWasm "already a member of family" failure', () => {
    const raw = new Error(
      'Abci query failed with code 6 - rpc error: code = Unknown desc = failed to execute message; ' +
        'message index: 0: node 52 is already a member of family 6: execute wasm contract failed',
    );
    expect(inviteWarningFromError(raw)).toBe('already-in-family');
  });

  it('maps a raw duplicate pending-invitation failure', () => {
    expect(inviteWarningFromError(new Error('a pending invitation already exists for node 7'))).toBe(
      'duplicate-pending',
    );
  });

  it('maps a raw non-existent / unbonding node failure', () => {
    expect(inviteWarningFromError(new Error('node 999 does not exist'))).toBe('non-existent');
  });

  it('returns undefined for unrelated errors', () => {
    expect(inviteWarningFromError(new Error('insufficient funds'))).toBeUndefined();
  });
});
