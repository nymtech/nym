"use client";

import {
  type EpochResponseData,
  useEpochContext,
} from "@/providers/EpochProvider";
import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { differenceInMinutes } from "date-fns";
import { useCallback, useEffect, useState } from "react";

const calculateMinutesRemaining = (epochEndTime: string) => {
  const endDate = new Date(epochEndTime);
  const difference = differenceInMinutes(endDate, new Date());
  return difference;
};

const NextEpochTime = () => {
  const { data, isLoading, isError, epochStatus } = useEpochContext();
  const [minutesRemaining, setMinutesRemaining] = useState(0);

  const updateState = useCallback((data: EpochResponseData) => {
    if (!data) return;
    const minutesRemaining = calculateMinutesRemaining(data.current_epoch_end);
    setMinutesRemaining(minutesRemaining);
  }, []);

  useEffect(() => {
    updateState(data);

    const interval = setInterval(() => {
      updateState(data);
    }, 30_000);

    return () => clearInterval(interval);
  }, [data, updateState]);

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
      {epochStatus === "pending" ? (
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
