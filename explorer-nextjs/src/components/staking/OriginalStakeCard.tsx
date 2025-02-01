"use client";

import { Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchOriginalStake } from "../../app/api";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";

const OriginalStakeCard = () => {
  const { address } = useNymClient();

  // Use React Query to fetch original stake
  const {
    data: originalStake = 0,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["originalStake", address],
    queryFn: () => fetchOriginalStake(address || ""),
    enabled: !!address, // Only fetch if address exists
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Original Stake">
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          Loading original stake...
        </Typography>
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Original Stake">
        <Typography
          variant="h3"
          sx={{ wordWrap: "break-word", maxWidth: "95%" }}
          color="error"
        >
          Failed to load original stake.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Original Stake">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        {`${formatBigNum(originalStake / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default OriginalStakeCard;
