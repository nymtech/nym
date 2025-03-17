import Box from "@mui/material/Box";
import LinearProgress from "@mui/material/LinearProgress";
import * as React from "react";

export interface IStaticProgressBarProps {
  color: string;
  value: number;
}

export const StaticProgressBar = (props: IStaticProgressBarProps) => {
  const { color, value } = props;

  return (
    <Box>
      <LinearProgress
        variant="determinate"
        value={value}
        sx={{
          height: 8,
          borderRadius: 4,
          backgroundColor: "#CAD6D7",
          "& .MuiLinearProgress-bar": {
            backgroundColor: color,
          },
        }}
      />
    </Box>
  );
};
