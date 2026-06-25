import React, { useMemo, useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { MockFamiliesContextProvider } from 'src/context/mocks/families';
import type { MockStore } from 'src/context/mocks/familiesMockState';

export interface WithFamiliesMockOptions {
  sender?: string;
  /** Factory so each story mount gets a fresh, isolated store. */
  makeStore?: () => MockStore;
  latencyMs?: number;
}

/**
 * Storybook decorator: wraps a story in a fresh QueryClient + the mock families
 * provider, so pages/flows run against the in-memory contract model (design D3, D7).
 */
export const withFamiliesMock =
  (options: WithFamiliesMockOptions = {}) =>
  // eslint-disable-next-line react/display-name
  (Story: React.ComponentType) => {
    const [client] = useState(
      () =>
        new QueryClient({
          defaultOptions: { queries: { retry: false, gcTime: 0 } },
        }),
    );
    const store = useMemo(() => options.makeStore?.(), []);
    return (
      <QueryClientProvider client={client}>
        <MockFamiliesContextProvider store={store} sender={options.sender} latencyMs={options.latencyMs ?? 150}>
          <Story />
        </MockFamiliesContextProvider>
      </QueryClientProvider>
    );
  };
