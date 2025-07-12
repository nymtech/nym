"use client";
import { fetchNoise } from "@/app/api";
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined";
import {
  Box,
  Skeleton,
  Stack,
  Tooltip,
  Typography,
  useTheme,
} from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import type { IPacketsAndStakingData } from "../../app/api/types";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { LineChart } from "../lineChart";
import { UpDownPriceIndicator } from "../price/UpDownPriceIndicator";

export const NoiseCard = () => {
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const [tooltipOpen, setTooltipOpen] = useState(false);

  const { data, isLoading, isError } = useQuery({
    queryKey: ["noise"],
    queryFn: fetchNoise,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });

  if (isLoading) {
    return (
      <ExplorerCard label="Mixnet traffic">
        <Stack gap={1}>
          <Skeleton variant="text" />
          <Skeleton variant="text" height={238} />
        </Stack>
      </ExplorerCard>
    );
  }

  // Don't display the card if there's an error or insufficient data
  if (isError || !data || !Array.isArray(data) || data.length < 10) {
    return null;
  }

  const todaysData = data[data.length - 2];
  const yesterdaysData = data[data.length - 3];

  const noiseLast24H =
    (todaysData?.total_packets_sent || 0) +
    (todaysData?.total_packets_received || 0);

  const noisePrevious24H =
    (yesterdaysData?.total_packets_sent || 0) +
    (yesterdaysData?.total_packets_received || 0);

  const formatNoiseVolume = (packets: number): string => {
    if (packets < 0) {
      throw new Error("Packets cannot be negative");
    }

    const BYTES_PER_PACKET = (2413 + 386) / 2;
    const totalBytes = packets * BYTES_PER_PACKET;
    const units = ["B", "KB", "MB", "GB", "TB", "PB"];

    let size = totalBytes;
    let unitIndex = 0;

    // Convert to the most appropriate unit
    for (; size >= 1024 && unitIndex < units.length - 1; unitIndex++) {
      size /= 1024;
    }

    return `${size.toFixed(2)} ${units[unitIndex]}`;
  };

  const formatedNoiseVolume = formatNoiseVolume(noiseLast24H);

  const calculatePercentageChange = (last24H: number, previous24H: number) => {
    if (previous24H === 0) {
      return previous24H;
    }

    const change = ((last24H - previous24H) / previous24H) * 100;

    return Number.parseFloat(change.toFixed(2));
  };

  const percentage = calculatePercentageChange(noiseLast24H, noisePrevious24H);

  const noiseLast24HFormatted = formatBigNum(noiseLast24H)?.toString() || "";

  const noiseLineGraphData = data
    .slice(0, -1)
    .map((item: IPacketsAndStakingData) => {
      return {
        date_utc: item?.date_utc,
        numericData:
          (item?.total_packets_sent || 0) + (item?.total_packets_received || 0),
      };
    })
    .filter((item) => item.numericData >= 2_500_000_000);

  const handleTooltipOpen = () => {
    setTooltipOpen(true);
  };

  const handleTooltipClose = () => {
    setTooltipOpen(false);
  };

  return (
    <ExplorerCard label="Mixnet traffic" sx={{ height: "100%" }}>
      <Box display={"flex"} gap={2} flexDirection={{ xs: "column", sm: "row" }}>
        <Typography
          variant="h4"
          sx={{
            color: isDarkMode ? "base.white" : "pine.950",
            wordWrap: "break-word",
            maxWidth: "95%",
          }}
        >
          {noiseLast24HFormatted}
        </Typography>
        <Tooltip
          placement="bottom"
          title={"Self reported noise volume"}
          open={tooltipOpen}
          onClose={handleTooltipClose}
          onClick={(e) => e.stopPropagation()}
          enterNextDelay={300}
          leaveDelay={200}
        >
          <Typography
            variant="h4"
            sx={{ color: "#8482FD", cursor: "pointer" }}
            onClick={handleTooltipOpen}
            onTouchStart={handleTooltipOpen}
            onMouseEnter={handleTooltipOpen}
            onMouseLeave={handleTooltipClose}
          >
            ({formatedNoiseVolume})
            <InfoOutlinedIcon sx={{ fontSize: 16 }} />
          </Typography>
        </Tooltip>
      </Box>
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
