"use client";
import type { ExplorerData, IPacketsAndStakingData } from "@/app/api";
import {
  CURRENT_EPOCH_REWARDS,
  HARBOURMASTER_API_MIXNODES_STATS,
} from "@/app/api/urls";
import { formatBigNum } from "@/app/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import { LineChart } from "../lineChart";

export const NetworkStakeCard = async () => {
  const epochRewards = await fetch(CURRENT_EPOCH_REWARDS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  const packetsAndStaking = await fetch(HARBOURMASTER_API_MIXNODES_STATS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  const epochRewardsData: ExplorerData["currentEpochRewardsData"] =
    await epochRewards.json();
  const packetsAndStakingData: ExplorerData["packetsAndStakingData"] =
    await packetsAndStaking.json();

  if (!epochRewardsData || !packetsAndStakingData) {
    return null;
  }

  const currentStake =
    Number(epochRewardsData.interval.staking_supply) / 1000000 || 0;

  const data = packetsAndStakingData.map((item: IPacketsAndStakingData) => {
    return {
      date_utc: item.date_utc,
      numericData: item.total_stake / 1000000,
    };
  });

  const stakeLineGraphData = {
    color: "#00CA33",
    label: "Total stake delegated in NYM",
    data,
  };

  const title = `${formatBigNum(currentStake)} NYM`;

  return (
    <ExplorerCard label="Current network stake" sx={{ height: "100%" }}>
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {title}
        </Typography>
        {stakeLineGraphData && <LineChart {...stakeLineGraphData} />}
      </Stack>
    </ExplorerCard>
  );
};
