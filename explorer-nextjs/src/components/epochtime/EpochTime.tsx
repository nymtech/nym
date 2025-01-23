"use client";

import { CURRENT_EPOCH } from "@/app/api/urls";
import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { addSeconds } from "date-fns";
import { format } from "date-fns";

// Fetch function for the next epoch
const fetchNextEpoch = async () => {
  const res = await fetch(CURRENT_EPOCH, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!res.ok) {
    throw new Error("Failed to fetch current epoch");
  }

  const data = await res.json();
  const dateTime = addSeconds(
    new Date(data.current_epoch_start),
    data.epoch_length.secs,
  );

  return { data, dateTime };
};

const NextEpochTime = () => {
  // Use React Query to fetch next epoch data
  const { data, isLoading, isError } = useQuery({
    queryKey: ["nextEpoch"], // Unique key for this query
    queryFn: fetchNextEpoch, // Fetch function
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is considered fresh for 60 seconds
  });

  if (isLoading) {
    return (
      <Stack direction="row" spacing={1}>
        <AccessTime />
        <Typography variant="h5" fontWeight="light">
          Loading next epoch...
        </Typography>
      </Stack>
    );
  }

  if (isError || !data) {
    return (
      <Stack direction="row" spacing={1}>
        <AccessTime />
        <Typography variant="h5" fontWeight="light">
          Failed to load next epoch.
        </Typography>
      </Stack>
    );
  }

  const formattedDate = format(data.dateTime, "HH:mm:ss");

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
