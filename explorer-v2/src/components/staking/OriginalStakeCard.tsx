"use client";

import { Skeleton, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchOriginalStake } from "../../app/api";
import { useNymClient } from "../../hooks/useNymClient";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import { useEnvironment } from "@/providers/EnvironmentProvider";

const OriginalStakeCard = () => {
  const { address } = useNymClient();
  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";
  const { environment } = useEnvironment();

  // Use React Query to fetch original stake
  const {
    data: originalStake = 0,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["originalStake", address],
    queryFn: () => fetchOriginalStake(address || "", environment),
    enabled: !!address, // Only fetch if address exists
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
  });


  if (!address) {
    return null; // Do not render if address is not available
  }

  if (isLoading) {
    return (
      <ExplorerCard label="Original Stake">
        <Skeleton variant="text" />
      </ExplorerCard>
    );
  }

  if (isError) {
    return (
      <ExplorerCard label="Original Stake">
        <Typography
          variant="h3"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load original stake.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Original Stake">
      <Typography
        variant="h3"
        sx={{
          color: isDarkMode ? "base.white" : "pine.950",
          wordWrap: "break-word",
          maxWidth: "95%",
        }}
      >
        {`${formatBigNum(originalStake / 1_000_000)} NYM`}
      </Typography>
    </ExplorerCard>
  );
};

export default OriginalStakeCard;
