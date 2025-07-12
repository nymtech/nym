"use client";
import { fetchNoise } from "@/app/api";
import { Box, Skeleton, Stack, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { ExplorerData, IPacketsAndStakingData } from "../../app/api/types";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { LineChart } from "../lineChart";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export const NetworkStakeCard = () => {
  const { environment } = useEnvironment();
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const {
    data: packetsAndStaking,
    isLoading: isStakingLoading,
    isError: isStakingError,
  } = useQuery({
    queryKey: ["noise", environment],
    queryFn: () => fetchNoise(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  if (isStakingLoading) {
    return (
      <ExplorerCard label="Current network stake">
        <Stack gap={1}>
          <Skeleton variant="text" />
          <Skeleton variant="text" height={238} />
        </Stack>
      </ExplorerCard>
    );
  }

  // Don't display the card if there's an error or insufficient data
  if (
    isStakingError ||
    !packetsAndStaking ||
    !Array.isArray(packetsAndStaking) ||
    packetsAndStaking.length < 10
  ) {
    return null;
  }

  const packetsAndStakingData: ExplorerData["packetsAndStakingData"] =
    packetsAndStaking;

  const lastTotalStake =
    packetsAndStaking[packetsAndStaking.length - 1]?.total_stake / 1_000_000;

  const startDate = new Date("2025-02-26").getTime(); // Convert to timestamp

  const data = packetsAndStakingData
    .slice(0, -1) // Exclude the last element
    .filter((item: IPacketsAndStakingData) => {
      const itemDate = new Date(item.date_utc).getTime(); // Convert each date to timestamp
      return itemDate >= startDate; // Compare timestamps
    })
    .map((item: IPacketsAndStakingData) => ({
      date_utc: item.date_utc,
      numericData: item.total_stake / 1000000,
    }));
  // .filter((item) => item.numericData >= 50_000_000);

  const stakeLineGraphData = {
    color: "#00CA33",
    label: "Total stake delegated in NYM",
    data,
  };

  const title = `${formatBigNum(lastTotalStake)} NYM`;

  return (
    <ExplorerCard label="Current network stake" sx={{ height: "100%" }}>
      <Stack>
        <Typography
          variant="h3"
          sx={{
            color: isDarkMode ? "base.white" : "pine.950",
            wordWrap: "break-word",
            maxWidth: "95%",
          }}
        >
          {title}
        </Typography>
        {stakeLineGraphData && (
          <Box height={225}>
            <LineChart {...stakeLineGraphData} />
          </Box>
        )}
      </Stack>
    </ExplorerCard>
  );
};
