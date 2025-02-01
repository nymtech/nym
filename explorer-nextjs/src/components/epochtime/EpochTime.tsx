"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQueryClient } from "@tanstack/react-query";
import { useQuery } from "@tanstack/react-query";
import { format, subSeconds } from "date-fns";
import { useCallback, useEffect, useState } from "react";
import { fetchCurrentEpoch } from "../../app/api";

const NextEpochTime = () => {
  const { data, isLoading, isError } = useQuery({
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000, // Keep refetching every 30s
    staleTime: 30000,
    refetchOnMount: true,
    keepPreviousData: false,
  });
  const queryClient = useQueryClient();

  const [hasEpochStarted, setHasEpochStarted] = useState(false);

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries(); // This will refetch ALL active queries
  }, [queryClient]);

  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return;

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime(); // Convert to timestamp

      const now = Date.now(); // Current time in ms
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus();

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval);
  });

  // Refetch all data on epoch change
  useEffect(() => {
    if (!hasEpochStarted) return;

    handleRefetch();

    const interval = setInterval(handleRefetch, 30000); // Run every 30s

    return () => clearInterval(interval);
  }, [hasEpochStarted, handleRefetch]);

  if (isLoading) {
    return (
      <Stack direction="row" spacing={1}>
        <AccessTime />
        <Typography variant="h5" fontWeight="light">
          Loading next epoch...
        </Typography>
      </Stack>
    );
  }

  if (isError || !data) {
    return (
      <Stack direction="row" spacing={1}>
        <AccessTime />
        <Typography variant="h5" fontWeight="light">
          Failed to load next epoch.
        </Typography>
      </Stack>
    );
  }

  return (
    <Stack direction="row" spacing={1}>
      <AccessTime />
      {hasEpochStarted ? (
        <Typography variant="h5" fontWeight="light">
          Waiting for next epoch to start...
        </Typography>
      ) : (
        <Typography variant="h5" fontWeight="light">
          Next epoch: {format(new Date(data.dateTime), "HH:mm:ss")}
        </Typography>
      )}
    </Stack>
  );
};

export default NextEpochTime;
