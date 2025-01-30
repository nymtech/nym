"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { fetchCurrentEpoch } from "../../app/api";

const NextEpochTime = () => {
  // Use React Query to fetch next epoch data
  const { data, isLoading, isError } = useQuery({
    queryKey: ["currentEpoch"], // Unique query key
    queryFn: fetchCurrentEpoch, // Fetch function
    refetchInterval: 30000, // Refetch every 30 seconds
    staleTime: 30000, // Data is considered fresh for 30 seconds
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
