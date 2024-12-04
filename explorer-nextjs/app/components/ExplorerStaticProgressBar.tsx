import * as React from "react";
import Box from "@mui/material/Box";
import LinearProgress from "@mui/material/LinearProgress";

export interface IExplorerStaticProgressBarProps {
  color: string;
  value: number;
}

export const ExplorerStaticProgressBar = (
  props: IExplorerStaticProgressBarProps
) => {
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
