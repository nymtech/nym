const delegationRoot = ['delegation'] as const;

export const delegationQueryKeys = {
  all: delegationRoot,
  /** Used when no client address so React Query never caches `summary('')`. */
  summaryDisabled: [...delegationRoot, 'summary', '__disabled__'] as const,
  summary: (clientAddress: string) => [...delegationRoot, 'summary', clientAddress] as const,
};
