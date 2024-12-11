"use client";
import type { ExplorerData, IPacketsAndStakingData } from "@/app/api";
import { Box, Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../Cards/ExplorerCard";
import ExplorerListItem from "../List/ListItem";
import { type ILineChartData, LineChart } from "../lineChart";

interface INetworkStakeCardProps {
  explorerData: ExplorerData | null;
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

  const stakeCard = {
    overTitle: "Current network stake",
    title: `${currentStake} NYM` || "",
    graph: stakeLineGraphData,
  };
  const subtitle = `${currentStake} NYM` || "";
  return (
    <div>
      <ExplorerCard title="Current network stake">
        <ExplorerListItem
          value={
            <Box mt={3} width={"100%"} height={"100%"}>
              <Typography
                variant="h3"
                sx={{ color: "pine.400", wordWrap: "break-word" }}
                maxWidth={"95%"}
              >
                {subtitle}
              </Typography>
              {stakeLineGraphData && (
                <LineChart
                  data={stakeLineGraphData.data}
                  color={stakeLineGraphData.color}
                  label={stakeLineGraphData.label}
                />
              )}
            </Box>
          }
        />
      </ExplorerCard>
    </div>
  );
};
