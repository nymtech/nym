"use client";

import {
  type EpochResponseData,
  useEpochContext,
} from "@/providers/EpochProvider";
import { Skeleton, Typography } from "@mui/material";
import { differenceInMinutes, format } from "date-fns";
import { useCallback, useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";
import EpochProgressBar from "../progressBars/EpochProgressBar";

const calulateProgress = (end: string) => {
  const endDate = new Date(end);
  const difference = differenceInMinutes(endDate, new Date());
  const progress = Math.max(0, 100 - (difference / 60) * 100);

  return progress;
};

const getStartEndTime = (start: string, end: string) => {
  const startDate = new Date(start);
  const endDate = new Date(end);

  const startTime = format(startDate, "HH:mm:ss");
  const endTime = format(endDate, "HH:mm:ss");

  return { startTime, endTime };
};

export const CurrentEpochCard = () => {
  const { data, isError, isLoading, epochStatus } = useEpochContext();
  const [startTime, setStartTime] = useState("");
  const [endTime, setEndTime] = useState("");
  const [progress, setProgress] = useState(0);

  const updateState = useCallback((data: NonNullable<EpochResponseData>) => {
    const { startTime, endTime } = getStartEndTime(
      data.current_epoch_start,
      data.current_epoch_end,
    );
    const progress = calulateProgress(data.current_epoch_end);

    setStartTime(startTime);
    setEndTime(endTime);
    setProgress(progress);
  }, []);

  useEffect(() => {
    if (!data) return;
    updateState(data);

    const intervalId = setInterval(() => {
      updateState(data);
    }, 30_000);

    return () => clearInterval(intervalId);
  }, [data, updateState]);

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
        <Typography variant="body3" fontWeight="light">
          Failed to load data
        </Typography>
      </ExplorerCard>
    );
  }

  if (!data) {
    return (
      <ExplorerCard label="Current mixnet epoch">
        <Typography variant="body3" fontWeight="light">
          No data available
        </Typography>
      </ExplorerCard>
    );
  }

  if (epochStatus === "pending") {
    return (
      <ExplorerCard label="Current mixnet epoch">
        <Typography variant="body3" fontWeight="light" height={80}>
          Waiting for next epoch to start...
        </Typography>
      </ExplorerCard>
    );
  }

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
