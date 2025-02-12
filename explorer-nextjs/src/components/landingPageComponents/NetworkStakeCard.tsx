"use client";
import { fetchNoise } from "@/app/api";
import { Box, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import type { ExplorerData, IPacketsAndStakingData } from "../../app/api/types";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { LineChart } from "../lineChart";

export const NetworkStakeCard = () => {
  const {
    data: packetsAndStaking,
    isLoading: isStakingLoading,
    isError: isStakingError,
  } = useQuery({
    queryKey: ["noise"],
    queryFn: fetchNoise,
  });

  if (isStakingLoading) {
    return (
      <ExplorerCard label="Current network stake">
        <Stack gap={1}>
          <Skeleton variant="text" />
          <Skeleton variant="text" height={238} />
        </Stack>
      </ExplorerCard>
    );
  }

  if (isStakingError || !packetsAndStaking) {
    return (
      <ExplorerCard label="Current network stake">
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Failed to load data
        </Typography>
        <Skeleton variant="text" height={238} />
      </ExplorerCard>
    );
  }

  const packetsAndStakingData: ExplorerData["packetsAndStakingData"] =
    packetsAndStaking;

  const lastTotalStake =
    packetsAndStaking[packetsAndStaking.length - 1]?.total_stake / 1000000;
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

  const title = `${formatBigNum(lastTotalStake)} NYM`;

  return (
    <ExplorerCard label="Current network stake" sx={{ height: "100%" }}>
      <Stack>
        <Typography
          variant="h3"
          sx={{ color: "pine.950", wordWrap: "break-word", maxWidth: "95%" }}
        >
          {title}
        </Typography>
        {stakeLineGraphData && (
          <Box height={225}>
            <LineChart {...stakeLineGraphData} />
          </Box>
        )}
      </Stack>
    </ExplorerCard>
  );
};
