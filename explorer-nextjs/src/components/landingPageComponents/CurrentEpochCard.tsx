"use client";

import { Box, Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import { subSeconds } from "date-fns";
import { useCallback, useEffect, useState } from "react";
import { fetchCurrentEpoch } from "../../app/api";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

export const CurrentEpochCard = () => {
  const queryClient = useQueryClient();

  const [hasEpochStarted, setHasEpochStarted] = useState(false);

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

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries(); // This will refetch ALL active queries
  }, [queryClient]);

  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return;

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime();

      const now = Date.now();
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus();

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval);
  });

  // Refetch all queries on epoch change
  useEffect(() => {
    if (!hasEpochStarted) return;

    handleRefetch();

    const interval = setInterval(handleRefetch, 30000);

    return () => clearInterval(interval);
  }, [hasEpochStarted, handleRefetch]);

  if (isLoading) {
    return (
      <ExplorerCard label="Current NGM epoch">
        <Skeleton variant="text" height={80} />
      </ExplorerCard>
    );
  }

  if (isError || !data) {
    return (
      <ExplorerCard label="Current NGM epoch">Failed to load data</ExplorerCard>
    );
  }

  const currentEpochStart = data.data.current_epoch_start || "";

  return (
    <ExplorerCard label="Current NGM epoch">
      <EpochProgressBar
        start={currentEpochStart}
        showEpoch={!hasEpochStarted}
        waitingForEpoch={
          hasEpochStarted ? (
            <Box mt={3}>
              <Typography variant="body3" fontWeight="light">
                Waiting for next epoch to start...
              </Typography>
            </Box>
          ) : undefined
        }
      />
    </ExplorerCard>
  );
};
