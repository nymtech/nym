"use client";
import { Stack } from "@mui/material";
import Box from "@mui/material/Box";
import { addHours, format } from "date-fns";
import React from "react";
import ListItem from "../list/ListItem";
import ProgressBar from "./ProgressBar";

export interface IDynamicProgressBarProps {
  start: string; // Start timestamp as ISO 8601 string
  showEpoch: boolean;
  waitingForEpoch?: React.ReactNode;
}

const EpochProgressBar = ({
  start,
  showEpoch,
  waitingForEpoch,
}: IDynamicProgressBarProps) => {
  const [progress, setProgress] = React.useState(0);

  const startDate = new Date(start);
  const endDate = addHours(new Date(start), 1);
  const startTime = format(startDate, "HH:mm dd-MM-yyyy");
  const endTime = format(endDate, "HH:mm dd-MM-yyyy");

  React.useEffect(() => {
    // Parse the start timestamp
    const startTime = new Date(start).getTime();
    const endTime = startTime + 60 * 60 * 1000; // Add 1 hour to the start time

    // Validate start timestamp
    if (Number.isNaN(startTime)) {
      console.error("Invalid start timestamp:", { start });
      return;
    }

    // Function to calculate progress
    const calculateProgress = () => {
      const currentTime = Date.now();
      if (currentTime < startTime) {
        return 0;
      }
      if (currentTime >= endTime) {
        return 100;
      }
      const elapsed = currentTime - startTime;
      const total = endTime - startTime;
      return (elapsed / total) * 100;
    };

    // Set initial progress and start timer
    setProgress(calculateProgress());
    const timer = setInterval(() => {
      setProgress(calculateProgress());
    }, 60000); // Update every minute (60000 milliseconds)

    // Cleanup on unmount
    return () => {
      clearInterval(timer);
    };
  }, [start]);

  return (
    <Box sx={{ width: "100%" }}>
      <ProgressBar value={progress} color="secondary" />
      {waitingForEpoch}

      {showEpoch && (
        <Box mt={3}>
          <Stack gap={0}>
            <ListItem row label="START" value={startTime} />
            <ListItem row label="END" value={endTime} />
          </Stack>
        </Box>
      )}
    </Box>
  );
};

export default EpochProgressBar;
