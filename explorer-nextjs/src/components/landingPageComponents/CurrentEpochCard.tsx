"use client";

import { Box, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { subSeconds } from "date-fns";
import { useEffect, useState } from "react";
import { fetchCurrentEpoch } from "../../app/api";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

export const CurrentEpochCard = () => {
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

  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return; // Ensure dateTime exists before running logic

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime(); // Convert to timestamp

      const now = Date.now(); // Current time in ms
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus(); // Check immediately on mount

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval); // Cleanup interval on unmount
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
