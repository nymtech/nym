"use client";

import { Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { ObservatoryBalance } from "../../app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "../../app/api/urls";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";

// Fetch balances based on the address
const fetchBalances = async (address: string): Promise<number> => {
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

  // Calculate total stake
  return (
    Number(balances.rewards.staking_rewards.amount) +
    Number(balances.delegated.amount)
  );
};

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
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is fresh for 60 seconds
  });

  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Total Stake">
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
