"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQueryClient } from "@tanstack/react-query";
import { useQuery } from "@tanstack/react-query";
import { subSeconds } from "date-fns";
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

  const [minutesRemaining, setMinutesRemaining] = useState(0);

  const handleRefetch = useCallback(() => {
    queryClient.invalidateQueries(); // This will refetch ALL active queries
  }, [queryClient]);

  // checking if new epoch has already started & update remaining minutes
  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return;

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime();

      const now = Date.now(); // Current time in ms
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus();

    // calculate remainting minutes
    const updateMinutesRemaining = () => {
      if (!data?.dateTime) return;

      const epochTime = new Date(data.dateTime).getTime();
      const now = Date.now();
      setMinutesRemaining(Math.max(0, Math.floor((epochTime - now) / 60000)));
    };
    updateMinutesRemaining();

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval);
  });

  // Refetch all data on epoch change
  useEffect(() => {
    if (!hasEpochStarted) return;

    handleRefetch();

    const interval = setInterval(handleRefetch, 10000); // Run every 10s

    return () => clearInterval(interval);
  }, [hasEpochStarted, handleRefetch]);

  if (isLoading) {
    return (
      <Stack direction="row" spacing={1}>
        <AccessTime />
        <Typography variant="h5" fontWeight="light">
          Loading next mixnet epoch...
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
          Waiting for next mixnet epoch to start...
        </Typography>
      ) : (
        <Typography variant="h5" fontWeight="light">
          Next mixnet epoch starts in: {minutesRemaining} min
        </Typography>
      )}
    </Stack>
  );
};

export default NextEpochTime;
