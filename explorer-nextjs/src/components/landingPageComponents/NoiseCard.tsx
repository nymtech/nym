"use client";
import type { ExplorerData, IPacketsAndStakingData } from "@/app/api";
import { formatBigNum } from "@/app/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import { useEffect, useState } from "react";
import ExplorerCard from "../cards/ExplorerCard";
import { type ILineChartData, LineChart } from "../lineChart";
import { UpDownPriceIndicator } from "../price/UpDownPriceIndicator";

interface INoiseCardProps {
  explorerData: ExplorerData | undefined;
}
export const NoiseCard = (props: INoiseCardProps) => {
  const { explorerData } = props;
  const [noiseLineGraphData, setNoiseLineGraphData] = useState<{
    color: string;
    label: string;
    data: ILineChartData[];
  }>();
  const noiseLast24H = explorerData
    ? explorerData.packetsAndStakingData[
        explorerData.packetsAndStakingData.length - 1
      ].total_packets_sent +
      explorerData.packetsAndStakingData[
        explorerData.packetsAndStakingData.length - 1
      ].total_packets_received
    : 0;

  const noisePrevious24H = explorerData
    ? explorerData.packetsAndStakingData[
        explorerData.packetsAndStakingData.length - 2
      ].total_packets_sent +
      explorerData.packetsAndStakingData[
        explorerData.packetsAndStakingData.length - 2
      ].total_packets_received
    : 0;

  const calculatePercentageChange = (last24H: number, previous24H: number) => {
    if (previous24H === 0) {
      return previous24H;
    }

    const change = ((last24H - previous24H) / previous24H) * 100;

    return Number.parseFloat(change.toFixed(2));
  };

  const percentage = calculatePercentageChange(noiseLast24H, noisePrevious24H);

  useEffect(() => {
    const getPacketsData = () => {
      const data: Array<ILineChartData> = [];
      explorerData?.packetsAndStakingData.map(
        (item: IPacketsAndStakingData) => {
          data.push({
            date_utc: item.date_utc,
            numericData: item.total_packets_sent + item.total_packets_received,
          });
        },
      );
      return data;
    };
    const noiseLineGraphData = {
      color: "#8482FD",
      label: "Total packets sent and received",
      data: getPacketsData(),
    };
    setNoiseLineGraphData(noiseLineGraphData);
  }, [explorerData]);

  const noiseCard = {
    overTitle: "Noise generated last 24h",
    title: formatBigNum(noiseLast24H) || "",
    upDownLine: {
      percentage: Math.abs(percentage) || 0,
      numberWentUp: percentage > 0,
    },
    graph: noiseLineGraphData,
  };
  const subtitle = formatBigNum(noiseLast24H)?.toString() || "";
  return (
    <ExplorerCard title="Noise generated last 24h">
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {subtitle}
        </Typography>
      </Stack>
      <UpDownPriceIndicator {...noiseCard.upDownLine} />
      {noiseLineGraphData && <LineChart {...noiseLineGraphData} />}
    </ExplorerCard>
  );
  // return <MonoCard {...noiseCard} />;
};
