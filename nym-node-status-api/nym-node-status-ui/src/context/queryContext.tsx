"use client";

import { type Client, createClient } from "@/client/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import type React from "react";
import { createContext, useContext, useRef } from "react";

interface State {
  client?: Client;
}

export const QueryContext = createContext<State>({
  client: undefined,
});

export const useQueryContext = (): React.ContextType<typeof QueryContext> =>
  useContext<State>(QueryContext);

export const QueryContextProvider = ({
  children,
}: {
  children: React.ReactNode | React.ReactNode[];
}) => {
  const openApiClient = useRef(
    createClient({ baseUrl: "https://mainnet-node-status-api.nymtech.cc" }),
  );
  const queryClient = useRef(new QueryClient());

  const state: State = {
    client: openApiClient.current,
  };

  return (
    <QueryContext.Provider value={state}>
      <QueryClientProvider client={queryClient.current}>
        {children}
        {/* Add devtools in development */}
        {process.env.NODE_ENV === "development" && (
          <ReactQueryDevtools initialIsOpen={false} />
        )}
      </QueryClientProvider>
    </QueryContext.Provider>
  );
};
