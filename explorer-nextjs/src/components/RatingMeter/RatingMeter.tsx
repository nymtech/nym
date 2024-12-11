import { LinearProgress, type LinearProgressProps } from "@mui/material";

const ProgressBar = ({
  value,
  color,
}: {
  value: number;
  color: LinearProgressProps["color"];
}) => {
  return (
    <LinearProgress
      variant="determinate"
      value={value}
      sx={{ height: 8, borderRadius: 5, width: "100%" }}
      color={color}
    />
  );
};
export default ProgressBar;
