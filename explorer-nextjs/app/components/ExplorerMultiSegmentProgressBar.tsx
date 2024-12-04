import React from "react";
import { Box } from "@mui/material";

export interface MultiSegmentProgressBarProps {
  values: { percentage: number; color: string }[]; // Array of percentage and color pairs
  height?: number; // Optional height, default is 8
  borderRadius?: number; // Optional border radius, default is 4
  backgroundColor?: string; // Optional background color for the bar, default is light gray
}

export const MultiSegmentProgressBar: React.FC<
  MultiSegmentProgressBarProps
> = ({ values, height = 8, borderRadius = 4, backgroundColor = "#CAD6D7" }) => {
  return (
    <Box
      sx={{
        display: "flex",
        width: "100%",
        height,
        borderRadius,
        overflow: "hidden",
        backgroundColor,
      }}
    >
      {values.map((value, index) => (
        <Box
          key={index}
          sx={{
            width: `${value.percentage}%`,
            backgroundColor: value.color,
          }}
        />
      ))}
    </Box>
  );
};
