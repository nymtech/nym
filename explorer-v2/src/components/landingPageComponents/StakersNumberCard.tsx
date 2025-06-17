"use client";
import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import { Skeleton, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import { useEnvironment } from "@/providers/EnvironmentProvider";

export const StakersNumberCard = () => {
  const { environment } = useEnvironment();

  const {
    data: nsApiNodes = [],
    isLoading: isNSApiNodesLoading,
    isError: isNSApiNodesError,
  } = useQuery({
    queryKey: ["nsApiNodes", environment],
    queryFn: () => fetchNSApiNodes(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  if (isNSApiNodesLoading) {
    return (
      <ExplorerCard label="Number of delegations">
        <Skeleton variant="text" height={90} />
      </ExplorerCard>
    );
  }

  if (isNSApiNodesError || !nsApiNodes) {
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

  const getActiveStakersNumber = (nodes: NS_NODE[]): number => {
    return nodes.reduce(
      (sum, node) => sum + (node.rewarding_details?.unique_delegations || 0),
      0
    );
  };
  const allStakers = getActiveStakersNumber(nsApiNodes);

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
