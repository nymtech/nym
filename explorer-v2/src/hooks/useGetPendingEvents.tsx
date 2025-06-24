import type { PendingEpochEventKind } from "@nymproject/contract-clients/Mixnet.types";
import { useQuery } from "@tanstack/react-query";
import { useEnvironment } from "../providers/EnvironmentProvider";

export const getEventsByAddress = (kind: PendingEpochEventKind, address: string) => {
  if ("delegate" in kind && kind.delegate.owner === address) {
    return {
      kind: "delegate" as const,
      mixId: kind.delegate.node_id,
      amount: kind.delegate.amount,
    };
  }

  if ("undelegate" in kind && kind.undelegate.owner === address) {
    return {
      kind: "undelegate" as const,
      mixId: kind.undelegate.node_id,
    };
  }

  return undefined;
};
export type PendingEvent = ReturnType<typeof getEventsByAddress>;

// Custom Hook for fetching pending events
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const usePendingEvents = (nymQueryClient: any, address: string | undefined) => {
  const { environment } = useEnvironment();

  return useQuery({
    queryKey: ["pendingEvents", address, environment], // Query key to uniquely identify this query
    queryFn: async () => {
      if (!nymQueryClient || !address) {
        throw new Error("Missing required dependencies");
      }

      const response = await nymQueryClient.getPendingEpochEvents({});
      const pendingEvents: PendingEvent[] = [];

      for (const e of response.events) {
        const event = getEventsByAddress(e.event.kind, address);
        if (event) {
          pendingEvents.push(event);
        }
      }

      return pendingEvents;
    },
    enabled: !!nymQueryClient && !!address, // Prevents execution if dependencies are missing
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });
};

export default usePendingEvents;
