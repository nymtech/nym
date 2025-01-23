"use client";

import type { ObservatoryBalance } from "@/app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "@/app/api/urls";
import { useNymClient } from "@/hooks/useNymClient";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";

// Fetch function to get the original stake
const fetchOriginalStake = async (address: string): Promise<number> => {
  const response = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch balances");
  }

  const balances: ObservatoryBalance = await response.json();

  // Return the delegated amount
  return Number(balances.delegated.amount);
};

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
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is fresh for 60 seconds
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Original Stake">
        <Typography variant="body2">Loading...</Typography>
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Original Stake">
        <Typography variant="body2" color="error">
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
