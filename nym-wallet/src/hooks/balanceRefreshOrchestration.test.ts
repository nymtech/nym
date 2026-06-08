import { runBalanceRefreshWithoutNestedLoading } from './balanceRefreshOrchestration';

describe('runBalanceRefreshWithoutNestedLoading', () => {
  it('delegates loading ownership to the caller by disabling nested loading toggles', async () => {
    const fetchBalance = jest.fn(async () => undefined);
    const fetchTokenAllocation = jest.fn(async () => undefined);

    await runBalanceRefreshWithoutNestedLoading(fetchBalance, fetchTokenAllocation);

    expect(fetchBalance).toHaveBeenCalledWith(false);
    expect(fetchTokenAllocation).toHaveBeenCalledWith(false, false);
  });
});
