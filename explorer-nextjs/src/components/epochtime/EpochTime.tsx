"use client";

import { AccessTime } from "@mui/icons-material";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useQueryClient } from "@tanstack/react-query";
import { format } from "date-fns";
import { fetchCurrentEpoch } from "../../app/api";

const NextEpochTime = () => {
  // Use React Query to fetch next epoch data
  const queryClient = useQueryClient();

  const { data, isLoading, isError } = useQuery({
    queryKey: ["currentEpoch"],
    queryFn: fetchCurrentEpoch,
    refetchInterval: 30000,
    enabled: true,
    staleTime: 30000,
    refetchOnMount: true, // Force UI update
    keepPreviousData: false, // Ensure new data updates UI
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
