import * as React from "react";
import Box from "@mui/material/Box";
import LinearProgress from "@mui/material/LinearProgress";
import { Typography } from "@mui/material";

export interface IExplorerProgressBarProps {
  title?: string;
  start: string; // Start timestamp as ISO 8601 string
  end: string; // End timestamp as ISO 8601 string
  showEpoch: boolean;
}

export const ExplorerProgressBar = (props: IExplorerProgressBarProps) => {
  const { start, end, showEpoch, title } = props;
  const [progress, setProgress] = React.useState(0);

  React.useEffect(() => {
    // Parse the timestamps
    const startTime = new Date(start).getTime();
    const endTime = new Date(end).getTime();

    // Validate timestamps
    if (isNaN(startTime) || isNaN(endTime)) {
      console.error("Invalid start or end timestamp:", { start, end });
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
  }, [start, end]);

  return (
    <Box sx={{ width: "100%" }}>
      {title && <Typography mb={2}>{title}</Typography>}

      <LinearProgress variant="determinate" value={progress} />
      {showEpoch && (
        <Box mt={2}>
          <Typography>Start: {new Date(start).toLocaleString()}</Typography>
          <Typography>End: {new Date(end).toLocaleString()}</Typography>
        </Box>
      )}
    </Box>
  );
};
