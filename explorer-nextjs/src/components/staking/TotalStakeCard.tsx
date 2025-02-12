"use client";

import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchBalances } from "../../app/api";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";

const TotalStakeCard = () => {
  const { address } = useNymClient();

  // Use React Query to fetch total stake
  const {
    data: totalStake = 0,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["totalStake", address],
    queryFn: () => fetchBalances(address || ""),
    enabled: !!address, // Only fetch if address exists
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Total Stake">
        <Skeleton variant="text" />
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Total Stake">
        <Typography variant="h3" color="error">
          Failed to load total stake.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Total Stake">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        {`${formatBigNum(totalStake / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default TotalStakeCard;
