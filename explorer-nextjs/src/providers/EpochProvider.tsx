"use client";

import { fetchCurrentEpoch } from "@/app/api";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { differenceInMilliseconds } from "date-fns";
import { createContext, useContext, useEffect, useState } from "react";

type EpochStatus = "active" | "pending";

export type EpochResponseData =
  | Awaited<ReturnType<typeof fetchCurrentEpoch>>
  | undefined;

type EpochContext = {
  epochStatus: EpochStatus;
  data: EpochResponseData;
  isError: boolean;
  isLoading: boolean;
};

const initialState = {
  epochStatus: "pending" as const,
  data: undefined,
  isError: false,
  isLoading: false,
};

const EpochContext = createContext<EpochContext>(initialState);

const checkIsEpochTimeValid = (epochEndTime: string) =>
  new Date(epochEndTime) >= new Date();

const calculateRefetchInterval = (epochEndTime: string) => {
  return differenceInMilliseconds(new Date(epochEndTime), new Date());
};

const useEpochContext = () => {
  const context = useContext(EpochContext);

  if (context === undefined) {
    throw new Error("useEpochContext must be used within a EpochProvider");
  }

  return context;
};

const EpochProvider = ({ children }: { children: React.ReactNode }) => {
  const [epochStatus, setEpochStatus] = useState<EpochStatus>("pending");

  const QueryClient = useQueryClient();

  const { data, isError, isLoading } = useQuery({
    refetchOnWindowFocus: true,
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: ({ state }) => {
      // refetchInterval can be set dynamically based on the current state

      // if there is no data, refetch in 30 secs
      if (!state.data) {
        return 30_000;
      }

      const isEpochTimeValid = checkIsEpochTimeValid(
        state.data.current_epoch_end.toString(),
      );

      // if epoch time is not valid (i.e current_time > epoch_start_time) refetch in 30 secs
      if (!isEpochTimeValid) {
        setEpochStatus("pending");
        return 30_000;
      }

      // if epoch time is valid, refetch based on the epoch end time
      const newRefetchInterval = calculateRefetchInterval(
        state.data.current_epoch_end.toString(),
      );

      setEpochStatus("active");
      return newRefetchInterval;
    },
  });

  useEffect(() => {
    const refreshQueries = async () => {
      await QueryClient.invalidateQueries({
        predicate: (query) => query.queryKey[0] !== "currentEpoch",
      });
    };

    // when new epoch starts, refresh all data
    if (epochStatus === "active") {
      refreshQueries();
    }
  }, [epochStatus, QueryClient]);

  const value = {
    epochStatus,
    data,
    isError,
    isLoading,
  };

  return (
    <EpochContext.Provider value={value}>{children}</EpochContext.Provider>
  );
};

export { EpochProvider, useEpochContext };
