import { Stack } from "@mui/material";
import Box from "@mui/material/Box";
import { addHours, differenceInMinutes, format } from "date-fns";
import ListItem from "../list/ListItem";
import ProgressBar from "../progressBar/ProgressBar";

export interface IDynamicProgressBarProps {
  start: string; // Start timestamp as ISO 8601 string
  showEpoch: boolean;
}

const EpochProgressBar = async ({
  start,
  showEpoch,
}: IDynamicProgressBarProps) => {
  const startDate = new Date(start);
  const endDate = addHours(new Date(start), 1);
  const startTime = format(startDate, "HH:mm dd-MM-yyyy");
  const endTime = format(endDate, "HH:mm dd-MM-yyyy");
  const totalEpochTime = differenceInMinutes(endDate, startDate);

  const progress =
    (differenceInMinutes(new Date(), startDate) / totalEpochTime) * 100;

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
