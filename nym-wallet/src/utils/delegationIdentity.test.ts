import {
  UNBONDED_NODE_IDENTITY_PREFIX,
  formatUnbondedNodeLabel,
  formatDelegationNodeIdentityForDisplay,
  formatPendingDelegationLinkLabel,
  isFullyUnbondedDelegation,
  isPendingUndelegateWithRegistryMiss,
  isUnbondedNodeIdentity,
} from './delegationIdentity';
import { buildPendingDelegateEvent, buildPendingUndelegateEvent } from './unbondedDelegation.fixture';

describe('delegationIdentity', () => {
  it('treats empty identity as unbonded', () => {
    expect(isUnbondedNodeIdentity('')).toBe(true);
    expect(isUnbondedNodeIdentity(undefined)).toBe(true);
    expect(isFullyUnbondedDelegation({ node_identity: '', mixnode_is_unbonding: false })).toBe(true);
  });

  it('treats unbonded prefix as fully unbonded', () => {
    const identity = `${UNBONDED_NODE_IDENTITY_PREFIX}42`;
    expect(isUnbondedNodeIdentity(identity)).toBe(true);
    expect(isFullyUnbondedDelegation({ node_identity: identity, mixnode_is_unbonding: false })).toBe(true);
  });

  it('does not treat bonded identity as unbonded', () => {
    const identity = '2Abcdefghijklmnopqrstuvwxyz1234567890';
    expect(isUnbondedNodeIdentity(identity)).toBe(false);
    expect(isFullyUnbondedDelegation({ node_identity: identity, mixnode_is_unbonding: true })).toBe(false);
  });

  it('formats unbonded node label with mix id', () => {
    expect(formatUnbondedNodeLabel(123)).toBe('Node unbonded (mix 123)');
  });

  it('formats unbonded identity for display using mix id', () => {
    expect(formatDelegationNodeIdentityForDisplay('unbonded:42', 42)).toBe('Node unbonded (mix 42)');
    expect(formatDelegationNodeIdentityForDisplay('2Abcdefghijklmnopqrstuvwxyz1234567890', 42)).toBe(
      '2Abcdefghijklmnopqrstuvwxyz1234567890',
    );
  });

  it('uses unbonded label only for pending undelegate registry misses', () => {
    const pendingDelegate = buildPendingDelegateEvent(`unbonded:${42}`);
    const pendingUndelegate = buildPendingUndelegateEvent(`unbonded:${42}`);

    expect(isPendingUndelegateWithRegistryMiss(pendingDelegate)).toBe(false);
    expect(isPendingUndelegateWithRegistryMiss(pendingUndelegate)).toBe(true);
  });

  it('formats pending delegate explorer link label by mix id when identity lookup missed', () => {
    expect(formatPendingDelegationLinkLabel('', 788)).toBe('Mix 788');
    expect(formatPendingDelegationLinkLabel('unbonded:788', 788)).toBe('Mix 788');
    expect(formatPendingDelegationLinkLabel('2Abcdefghijklmnopqrstuvwxyz1234567890', 788)).toBe('2Abcde...567890');
  });
});
