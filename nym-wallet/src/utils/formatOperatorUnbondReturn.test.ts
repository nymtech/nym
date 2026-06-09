import { TBondedNode } from 'src/context/bonding';
import { formatCoinDisplay, formatOperatorUnbondReturn } from './formatOperatorUnbondReturn';

const mixnodeWithRewards = {
  bond: { denom: 'nym', amount: '1000' },
  operatorRewards: { denom: 'nym', amount: '250.5' },
} as TBondedNode;

const nymNodeWithRewards = {
  bond: { denom: 'nym', amount: '1000' },
  operatorRewards: { denom: 'nym', amount: '100' },
} as TBondedNode;

const gateway = {
  bond: { denom: 'nym', amount: '500' },
} as TBondedNode;

describe('formatOperatorUnbondReturn', () => {
  it('sums pledge and compounded operator rewards for mixnodes', () => {
    const result = formatOperatorUnbondReturn(mixnodeWithRewards);
    expect(result.hasCompoundedRewards).toBe(true);
    expect(result.pledge.amount).toBe('1000');
    expect(result.operatorRewards?.amount).toBe('250.5');
    expect(result.total.amount).toBe('1250.5');
    expect(formatCoinDisplay(result.total)).toBe('1250.5 NYM');
  });

  it('sums pledge and compounded operator rewards for nym nodes', () => {
    const result = formatOperatorUnbondReturn(nymNodeWithRewards);
    expect(result.hasCompoundedRewards).toBe(true);
    expect(result.total.amount).toBe('1100');
  });

  it('returns pledge only when operator rewards are zero', () => {
    const result = formatOperatorUnbondReturn({
      ...mixnodeWithRewards,
      operatorRewards: { denom: 'nym', amount: '0' },
    } as TBondedNode);
    expect(result.hasCompoundedRewards).toBe(false);
    expect(result.total.amount).toBe('1000');
    expect(result.operatorRewards).toBeNull();
  });

  it('returns pledge only for gateways without operator rewards', () => {
    const result = formatOperatorUnbondReturn(gateway);
    expect(result.hasCompoundedRewards).toBe(false);
    expect(result.total.amount).toBe('500');
    expect(result.operatorRewards).toBeNull();
  });

  it('preserves the raw amount string and sets parseError when bond amount is malformed', () => {
    const result = formatOperatorUnbondReturn({
      bond: { denom: 'nym', amount: 'not-a-number' },
    } as TBondedNode);
    expect(result.pledge.amount).toBe('not-a-number');
    expect(result.parseError).toMatch(/Could not parse amount/);
    expect(result.hasCompoundedRewards).toBe(false);
  });

  it('sets parseError and treats malformed reward as zero when reward amount is unparseable', () => {
    const result = formatOperatorUnbondReturn({
      ...mixnodeWithRewards,
      operatorRewards: { denom: 'nym', amount: 'NaN' },
    } as TBondedNode);
    expect(result.operatorRewards).toBeNull();
    expect(result.hasCompoundedRewards).toBe(false);
    expect(result.parseError).toMatch(/Could not/);
  });
});
