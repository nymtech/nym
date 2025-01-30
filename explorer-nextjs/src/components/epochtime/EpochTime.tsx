"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { addHours, format, isAfter } from "date-fns";
import { useEffect, useState } from "react";
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

  const [hasEpochStarted, setHasEpochStarted] = useState(false);

  useEffect(() => {
    const checkEpochStatus = () => {
      if (data?.dateTime) {
        const oneHourLater = addHours(new Date(data.dateTime), 1);
        setHasEpochStarted(isAfter(Date.now(), oneHourLater));
      }
    };

    checkEpochStatus(); // Check immediately on mount

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s

    return () => clearInterval(interval); // Cleanup on unmount
  }, [data?.dateTime]);

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
          Next epoch: {format(data.dateTime, "HH:mm:ss")}
        </Typography>
      )}
    </Stack>
  );
};

export default NextEpochTime;
