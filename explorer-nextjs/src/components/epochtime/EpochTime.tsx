"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { addHours, format, subSeconds } from "date-fns";
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
  console.log("hasEpochStarted :>> ", hasEpochStarted);

  useEffect(() => {
    const checkEpochStatus = () => {
      if (!data?.dateTime) return; // Ensure dateTime exists before running logic

      const oneHourLater = subSeconds(new Date(data.dateTime), 30).getTime(); // Convert to timestamp

      console.log(
        "oneHourLater :>> ",
        format(new Date(oneHourLater), "HH:mm:ss"),
      );

      const now = Date.now(); // Current time in ms
      console.log("now :>> ", format(new Date(now), "HH:mm:ss"));
      setHasEpochStarted(now >= oneHourLater);
    };

    checkEpochStatus(); // Check immediately on mount

    const interval = setInterval(checkEpochStatus, 30000); // Check every 30s, regardless of data updates

    return () => clearInterval(interval); // Cleanup interval on unmount
  }); //  No dependencies → Runs every 30s even if `data` doesn’t change

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
