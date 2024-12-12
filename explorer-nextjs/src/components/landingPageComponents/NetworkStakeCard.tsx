"use client";
import type { ExplorerData, IPacketsAndStakingData } from "@/app/api";
import { Stack, Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";
import { type ILineChartData, LineChart } from "../lineChart";

interface INetworkStakeCardProps {
  explorerData: ExplorerData | undefined;
}
export const NetworkStakeCard = (props: INetworkStakeCardProps) => {
  const { explorerData } = props;
  const [stakeLineGraphData, setStakeLineGraphData] = useState<{
    color: string;
    label: string;
    data: ILineChartData[];
  }>();
  const currentStake =
    Number(explorerData?.currentEpochRewardsData.interval.staking_supply) /
      1000000 || 0;

  useEffect(() => {
    const getStakeData = () => {
      const data: Array<ILineChartData> = [];
      explorerData?.packetsAndStakingData.map(
        (item: IPacketsAndStakingData) => {
          data.push({
            date_utc: item.date_utc,
            numericData: item.total_stake / 1000000,
          });
        },
      );
      return data;
    };
    const stakeLineGraphData = {
      color: "#00CA33",
      label: "Total stake delegated in NYM",
      data: getStakeData(),
    };
    setStakeLineGraphData(stakeLineGraphData);
  }, [explorerData]);

  const title = `${currentStake} NYM` || "";
  return (
    <ExplorerCard title="Current network stake">
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
