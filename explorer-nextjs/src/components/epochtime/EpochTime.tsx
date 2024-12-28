import { CURRENT_EPOCH } from "@/app/api/urls";
import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { addSeconds } from "date-fns";
import { format } from "date-fns";

export const fetcNextEpoch = async () => {
  const res = await fetch(CURRENT_EPOCH, {
    next: { revalidate: 60 },
  });
  const data = await res.json();

  const dateTime = addSeconds(
    new Date(data.current_epoch_start),
    data.epoch_length.secs,
  );

  return { data, dateTime };
};

const NextEpochTime = async () => {
  const epoch = await fetcNextEpoch();
  const formattedDate = format(epoch.dateTime, "HH:mm:ss");

  return (
    <Stack direction="row" spacing={1}>
      <AccessTime />
      <Typography variant="h5" fontWeight="light">
        Next epoch: {formattedDate}
      </Typography>
    </Stack>
  );
};

export default NextEpochTime;
