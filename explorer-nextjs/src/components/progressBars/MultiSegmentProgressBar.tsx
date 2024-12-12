import { Box } from "@mui/material";
import type React from "react";

export interface MultiSegmentProgressBarProps {
  values: { percentage: number; color: string }[]; // Array of percentage and color pairs
  // Optional border radius, default is 4
  backgroundColor?: string; // Optional background color for the bar, default is light gray
}

export const MultiSegmentProgressBar: React.FC<
  MultiSegmentProgressBarProps
> = ({ values, backgroundColor = "#CAD6D7" }) => {
  return (
    <Box
      sx={{
        display: "flex",
        width: "100%",
        height: 8,
        borderRadius: 4,
        overflow: "hidden",
        backgroundColor,
      }}
    >
      {values.map((value) => (
        <Box
          key={value.color}
          sx={{
            width: `${value.percentage}%`,
            backgroundColor: value.color,
          }}
        />
      ))}
    </Box>
  );
};
