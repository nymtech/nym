"use client";

import { Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchTotalStakerRewards } from "../../app/api";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";

const TotalRewardsCard = () => {
  const { address } = useNymClient();

  // Use React Query to fetch total staker rewards
  const {
    data: totalStakerRewards = 0,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["totalStakerRewards", address],
    queryFn: () => fetchTotalStakerRewards(address || ""),
    enabled: !!address, // Only fetch if address exists
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is fresh for 60 seconds
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Total Rewards">
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          Loading...
        </Typography>
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Total Rewards">
        <Typography variant="h3" color="error">
          Failed to load total rewards.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Total Rewards">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        {`${formatBigNum(totalStakerRewards / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default TotalRewardsCard;
