"use client";

import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { differenceInMinutes, format } from "date-fns";
import { fetchCurrentEpoch } from "../../app/api";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

export const CurrentEpochCard = () => {
  // Use React Query to fetch data
  const { data, isError, isLoading } = useQuery({
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000,
  });

  if (isLoading) {
    return (
      <ExplorerCard label="Current mixnet epoch">
        <Skeleton variant="text" height={80} />
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Current mixnet epoch">
        Failed to load data
      </ExplorerCard>
    );
  }

  if (!data) {
    return (
      <ExplorerCard label="Current mixnet epoch">
        No data available
      </ExplorerCard>
    );
  }

  const isEpochTimeValid = new Date(data.dateTime) > new Date();

  if (!isEpochTimeValid) {
    return (
      <ExplorerCard label="Current mixnet epoch">
        <Typography variant="body3" fontWeight="light">
          Waiting for next epoch to start...
        </Typography>
      </ExplorerCard>
    );
  }

  const currentEpochStart = data.data.current_epoch_start || "";

  const startDate = new Date(currentEpochStart);
  const endDate = new Date(data.dateTime);

  const startTime = format(startDate, "HH:mm:ss");
  const endTime = format(endDate, "HH:mm:ss");

  const difference = differenceInMinutes(endDate, new Date());

  const progress = Math.max(0, 100 - (difference / 60) * 100);
  return (
    <ExplorerCard label="Current mixnet epoch">
      <EpochProgressBar
        startTime={startTime}
        endTime={endTime}
        progress={progress}
      />
    </ExplorerCard>
  );
};
