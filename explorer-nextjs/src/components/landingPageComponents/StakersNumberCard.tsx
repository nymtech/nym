"use client";
import { fetchObservatoryNodes } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";

export const RewardsCard = () => {
  const {
    data: observatoryNodes,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["observatoryNodes"],
    queryFn: () => fetchObservatoryNodes(),
  });

  if (isLoading) {
    return (
      <ExplorerCard label="NYM Delegations">
        <Skeleton variant="text" height={90} />
      </ExplorerCard>
    );
  }

  if (isError || !observatoryNodes) {
    return (
      <ExplorerCard label="NYM Delegations">
        <Typography variant="h3" sx={{ color: "pine.950" }}>
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
  const allStakers = getActiveStakersNumber(observatoryNodes);

  return (
    <ExplorerCard label="NYM Delegations">
      <Typography variant="h3" sx={{ color: "pine.950" }}>
        {allStakers}
      </Typography>
    </ExplorerCard>
  );
};
