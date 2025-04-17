"use client";
import { fetchObservatoryNodes } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import { Skeleton, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";

export const StakersNumberCard = () => {
  const {
    data: nymNodes,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: () => fetchObservatoryNodes(),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  if (isLoading) {
    return (
      <ExplorerCard label="Number of delegations">
        <Skeleton variant="text" height={90} />
      </ExplorerCard>
    );
  }

  if (isError || !nymNodes) {
    return (
      <ExplorerCard label="Number of delegations">
        <Typography
          variant="h5"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  const getActiveStakersNumber = (nodes: IObservatoryNode[]): number => {
    return nodes.reduce(
      (sum, node) => sum + node.rewarding_details.unique_delegations,
      0,
    );
  };
  const allStakers = getActiveStakersNumber(nymNodes);

  return (
    <ExplorerCard label="Number of delegations">
      <Typography
        variant="h3"
        sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
      >
        {allStakers}
      </Typography>
    </ExplorerCard>
  );
};
