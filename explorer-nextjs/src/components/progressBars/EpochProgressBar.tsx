"use client";
import { Stack } from "@mui/material";
import Box from "@mui/material/Box";
import { addHours, format } from "date-fns";
import React from "react";
import ListItem from "../list/ListItem";
import ProgressBar from "./ProgressBar";

export interface IDynamicProgressBarProps {
  startTime: string;
  endTime: string;
  progress: number;
}

const EpochProgressBar = ({
  startTime,
  endTime,
  progress,
}: IDynamicProgressBarProps) => {
  return (
    <Box sx={{ width: "100%" }}>
      <ProgressBar value={progress} color="secondary" />

      <Box mt={3}>
        <Stack gap={0}>
          <ListItem row label="START" value={startTime} />
          <ListItem row label="END" value={endTime} />
        </Stack>
      </Box>
    </Box>
  );
};

export default EpochProgressBar;
