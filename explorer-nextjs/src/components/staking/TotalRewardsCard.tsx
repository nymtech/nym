import type { ObservatoryBalance } from "@/app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "@/app/api/urls";
import { useNymClient } from "@/hooks/useNymClient";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";

const TotalRewardsCard = () => {
  const [totalStakerRewards, setTotalStakerRewards] = useState<string>();
  const { nymClient, address } = useNymClient();

  useEffect(() => {
    if (!nymClient || !address) return;

    const fetchBalances = async () => {
      const data = await fetch(`${DATA_OBSERVATORY_BALANCES_URL}/${address}`, {
        headers: {
          Accept: "application/json",
          "Content-Type": "application/json; charset=utf-8",
        },
        next: { revalidate: 60 },
        // refresh event list cache at given interval
      });
      const balances: ObservatoryBalance = await data.json();
      console.log("balances :>> ", balances);
      const stakerRewards = formatBigNum(
        +balances.rewards.staking_rewards.amount / 1_000_000,
      );

      return setTotalStakerRewards(stakerRewards);
    };

    fetchBalances();
  }, [address, nymClient]);

  if (!address) {
    return null;
  }
  return (
    <ExplorerCard label="Total Rewards">
      <Typography
        variant="h3"
        sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
      >
        {totalStakerRewards || 0} NYM
      </Typography>
    </ExplorerCard>
  );
};

export default TotalRewardsCard;
