"use client";

import { Skeleton, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchTotalStakerRewards } from "../../app/api";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { useEnvironment } from "@/providers/EnvironmentProvider";

const TotalRewardsCard = () => {
  const { address } = useNymClient();
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const { environment } = useEnvironment();

  // Use React Query to fetch total rewards
  const {
    data: totalStakerRewards = 0,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["totalStakerRewards", address, environment],
    queryFn: () => fetchTotalStakerRewards(address || "", environment),
    enabled: !!address, // Only fetch if address exists
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Total Rewards">
        <Skeleton variant="text" />
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Total Rewards">
        <Typography
          variant="h3"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load total rewards.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Total Rewards">
      <Typography
        variant="h3"
        sx={{
          color: isDarkMode ? "base.white" : "pine.950",
          wordWrap: "break-word",
          maxWidth: "95%",
        }}
      >
        {`${formatBigNum(totalStakerRewards / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default TotalRewardsCard;
