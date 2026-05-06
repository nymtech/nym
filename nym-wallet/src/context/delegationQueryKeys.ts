export const delegationQueryKeys = {
  all: ['delegation'] as const,
  summary: (clientAddress: string) => [...delegationQueryKeys.all, 'summary', clientAddress] as const,
};
