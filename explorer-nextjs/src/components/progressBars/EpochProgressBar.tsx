import { Stack } from "@mui/material";
import Box from "@mui/material/Box";
import { addHours, differenceInMinutes, format } from "date-fns";
import * as React from "react";
import ListItem from "../list/ListItem";
import ProgressBar from "../progressBar/ProgressBar";

export interface IDynamicProgressBarProps {
  start: string; // Start timestamp as ISO 8601 string
  showEpoch: boolean;
}

const EpochProgressBar = ({ start, showEpoch }: IDynamicProgressBarProps) => {
  const startTime = format(new Date(start), "HH:mm dd-MM-yyyy");
  const endTime = format(addHours(new Date(start), 1), "HH:mm dd-MM-yyyy");
  const totalEpochTime = differenceInMinutes(
    new Date(endTime),
    new Date(startTime),
  );

  const progress =
    (differenceInMinutes(new Date(), startTime) / totalEpochTime) * 100;

  console.log(progress);

  return (
    <Box sx={{ width: "100%" }}>
      <ProgressBar value={progress} color="secondary" />

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
