"use client";

import { Typography } from "@mui/material";
import Box from "@mui/material/Box";
import LinearProgress from "@mui/material/LinearProgress";
import * as React from "react";

export interface IDynamicProgressBarProps {
  overTitle?: string;
  start: string; // Start timestamp as ISO 8601 string
  showEpoch: boolean;
}

export const DynamicProgressBar = (props: IDynamicProgressBarProps) => {
  const { start, showEpoch, overTitle } = props;
  const [progress, setProgress] = React.useState(0);

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

  // Helper function to format date
  const formatDate = (timestamp: number) => {
    const date = new Date(timestamp);
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const month = String(date.getMonth() + 1).padStart(2, "0"); // Months are 0-based
    const year = date.getFullYear();
    return `${hours}:${minutes}, ${day}/${month}/${year}`;
  };

  const startTime = new Date(start).getTime();
  const endTime = startTime + 60 * 60 * 1000;

  return (
    <Box sx={{ width: "100%" }}>
      {overTitle && (
        <Typography fontSize={14} mb={2} textTransform={"uppercase"}>
          {overTitle}
        </Typography>
      )}

      <LinearProgress
        variant="determinate"
        value={progress}
        sx={{
          backgroundColor: "#CAD6D7",
          "& .MuiLinearProgress-bar": {
            backgroundColor: "#14E76F",
          },
        }}
      />
      {showEpoch && (
        <Box mt={2}>
          <Box display={"flex"} justifyContent={"space-between"}>
            <Typography fontSize={14} textTransform={"uppercase"}>
              START:
            </Typography>
            <Typography fontSize={14}>
              {startTime ? formatDate(startTime) : ""}
            </Typography>
          </Box>
          <Box display={"flex"} justifyContent={"space-between"}>
            <Typography fontSize={14} textTransform={"uppercase"}>
              END:
            </Typography>
            <Typography fontSize={14}>
              {endTime ? formatDate(endTime) : ""}
            </Typography>
          </Box>
        </Box>
      )}
    </Box>
  );
};
