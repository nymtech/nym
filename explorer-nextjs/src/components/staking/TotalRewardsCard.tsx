import type { ObservatoryBalance } from "@/app/api/types";
import { DATA_OBSERVATORY_BALANCES_URL } from "@/app/api/urls";
import { useNymClient } from "@/hooks/useNymClient";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";

const TotalRewardsCard = () => {
  const [totalStakerRewards, setTotalStakerRewards] = useState<number>(0);
  const { address } = useNymClient();

  useEffect(() => {
    if (!address) return;

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

      return setTotalStakerRewards(balances.rewards.staking_rewards.amount);
    };

    fetchBalances();
  }, [address]);

  if (!address) {
    return null;
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
