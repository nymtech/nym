"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchCurrentEpoch } from "../../app/api";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

export const CurrentEpochCard = () => {
  // Use React Query to fetch data
  const { data, isError, isLoading } = useQuery({
    enabled: true,
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000,
    staleTime: 30000,
    refetchOnMount: true, // Force UI update
    keepPreviousData: false, // Ensure new data updates UI
  });

  if (isLoading) {
    return <ExplorerCard label="Current NGM epoch">Loading...</ExplorerCard>;
  }

  if (isError || !data) {
    return (
      <ExplorerCard label="Current NGM epoch">Failed to load data</ExplorerCard>
    );
  }

  const currentEpochStart = data.data.current_epoch_start || "";

  return (
    <ExplorerCard label="Current NGM epoch">
      <EpochProgressBar start={currentEpochStart} showEpoch={true} />
    </ExplorerCard>
  );
};
