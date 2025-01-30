import { Box, Stack, Typography } from "@mui/material";
import type { IPacketsAndStakingData } from "../../app/api/types";
import { HARBOURMASTER_API_MIXNODES_STATS } from "../../app/api/urls";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { LineChart } from "../lineChart";
import { UpDownPriceIndicator } from "../price/UpDownPriceIndicator";

export const NoiseCard = async () => {
  const response = await fetch(HARBOURMASTER_API_MIXNODES_STATS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  const data: IPacketsAndStakingData[] = await response.json();

  if (!data) {
    return null;
  }

  const todaysData = data[data.length - 1];
  const yesterdaysData = data[data.length - 2];

  const noiseLast24H =
    todaysData.total_packets_sent + todaysData.total_packets_received;
  const noisePrevious24H =
    yesterdaysData.total_packets_sent + yesterdaysData.total_packets_received;

  const calculatePercentageChange = (last24H: number, previous24H: number) => {
    if (previous24H === 0) {
      return previous24H;
    }

    const change = ((last24H - previous24H) / previous24H) * 100;

    return Number.parseFloat(change.toFixed(2));
  };

  const percentage = calculatePercentageChange(noiseLast24H, noisePrevious24H);

  const noiseLast24HFormatted = formatBigNum(noiseLast24H)?.toString() || "";

  const noiseLineGraphData = data.map((item: IPacketsAndStakingData) => {
    return {
      date_utc: item.date_utc,
      numericData: item.total_packets_sent + item.total_packets_received,
    };
  });

  return (
    <ExplorerCard label="Noise generated last 24h" sx={{ height: "100%" }}>
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {noiseLast24HFormatted}
        </Typography>
      </Stack>
      <UpDownPriceIndicator
        percentage={Math.abs(percentage) || 0}
        numberWentUp={percentage > 0}
      />
      {noiseLineGraphData && (
        <Box height={225}>
          <LineChart
            color="#8482FD"
            label="Total packets sent and received"
            data={noiseLineGraphData}
          />
        </Box>
      )}
    </ExplorerCard>
  );
};
