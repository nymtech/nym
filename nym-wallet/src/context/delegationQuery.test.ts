import { getAllPendingDelegations, getDelegationSummary } from 'src/requests';
import { fetchDelegationSummaryQuery } from './delegationQuery';

jest.mock('src/utils', () => ({
  decCoinToDisplay: jest.fn((coin: { amount: string; denom: string }) => coin),
}));

jest.mock('src/requests', () => ({
  getDelegationSummary: jest.fn(),
  getAllPendingDelegations: jest.fn(),
}));

const mockedGetDelegationSummary = getDelegationSummary as jest.MockedFunction<typeof getDelegationSummary>;
const mockedGetAllPendingDelegations = getAllPendingDelegations as jest.MockedFunction<typeof getAllPendingDelegations>;

describe('fetchDelegationSummaryQuery', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('includes unbonded delegations in totals and keeps pending events on new nodes separate', async () => {
    mockedGetDelegationSummary.mockResolvedValue({
      delegations: [
        {
          mix_id: 42,
          node_identity: 'unbonded:42',
          amount: { amount: '1000000', denom: 'unym' },
          unclaimed_rewards: { amount: '50000', denom: 'unym' },
          mixnode_is_unbonding: null,
        },
      ],
      total_delegations: { amount: '1000000', denom: 'unym' },
      total_rewards: { amount: '50000', denom: 'unym' },
    } as unknown as Awaited<ReturnType<typeof getDelegationSummary>>);

    mockedGetAllPendingDelegations.mockResolvedValue([
      {
        node_identity: 'new-node-identity',
        event: { mix_id: 99, kind: 'Delegate', address: 'n1test' },
      },
    ] as Awaited<ReturnType<typeof getAllPendingDelegations>>);

    const result = await fetchDelegationSummaryQuery();

    expect(result.delegations).toHaveLength(2);
    expect(result.delegations[0]).toMatchObject({ mix_id: 42, node_identity: 'unbonded:42' });
    expect(result.pendingDelegations).toHaveLength(1);
    expect(result.totalDelegations.amount).toBe('1000000');
    expect(result.totalRewards.amount).toBe('50000');
    expect(result.totalDelegationsAndRewards.amount).toBe('1050000.000000');
  });
});
