export async function runBalanceRefreshWithoutNestedLoading(
  fetchBalance: (manageLoading?: boolean) => Promise<void>,
  fetchTokenAllocation: (isBackgroundPoll?: boolean, manageLoading?: boolean) => Promise<void>,
): Promise<void> {
  await fetchBalance(false);
  await fetchTokenAllocation(false, false);
}
