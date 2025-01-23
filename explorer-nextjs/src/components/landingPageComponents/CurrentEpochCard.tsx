"use client";

import type { CurrentEpochData } from "@/app/api";
import { CURRENT_EPOCH } from "@/app/api/urls";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

// Fetch function
const fetchCurrentEpoch = async (): Promise<CurrentEpochData> => {
  const response = await fetch(CURRENT_EPOCH, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch current epoch data");
  }

  return response.json();
};

export const CurrentEpochCard = () => {
  // Use React Query to fetch data
  const { data, isError, isLoading } = useQuery({
    queryKey: ["currentEpoch"], // Unique query key
    queryFn: fetchCurrentEpoch, // Fetch function
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is considered fresh for 60 seconds
  });

  if (isLoading) {
    return <ExplorerCard label="Current NGM epoch">Loading...</ExplorerCard>;
  }

  if (isError || !data) {
    return (
      <ExplorerCard label="Current NGM epoch">Failed to load data</ExplorerCard>
    );
  }

  const currentEpochStart = data.current_epoch_start || "";

  return (
    <ExplorerCard label="Current NGM epoch">
      <EpochProgressBar start={currentEpochStart} showEpoch={true} />
    </ExplorerCard>
  );
};
