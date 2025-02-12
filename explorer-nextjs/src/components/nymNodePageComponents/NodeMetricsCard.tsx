"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchEpochRewards, fetchNodeInfo } from "../../app/api";

import { Skeleton, Typography } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  id: number; // Node ID
}

export const NodeMetricsCard = ({ id }: INodeMetricsCardProps) => {
  const {
    data: epochRewardsData,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
  });

  // Fetch node information
  const {
    data: nodeInfo,
    isLoading: isNodeLoading,
    isError: isNodeError,
  } = useQuery({
    queryKey: ["nodeInfo", id],
    queryFn: () => fetchNodeInfo(id),
  });

  if (isEpochLoading || isNodeLoading) {
    return (
      <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isEpochError || isNodeError || !nodeInfo || !epochRewardsData) {
    return (
      <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  // Function to calculate active set probability
  const getActiveSetProbability = (
    totalStake: number,
    stakeSaturationPoint: string,
  ): string => {
    const saturation = Number.parseFloat(stakeSaturationPoint);

    if (Number.isNaN(saturation) || saturation <= 0) {
      throw new Error("Invalid stake saturation point provided");
    }

    const ratio = (totalStake / saturation) * 100;

    if (ratio > 70) {
      return "High";
    }
    if (ratio >= 40 && ratio <= 70) {
      return "Medium";
    }
    return "Low";
  };

  const activeSetProb = getActiveSetProbability(
    nodeInfo.total_stake,
    epochRewardsData.interval.stake_saturation_point,
  );

  return (
    <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Node ID."
        value={nodeInfo.node_id.toString()}
      />
      <ExplorerListItem
        row
        divider
        label="Host"
        value={nodeInfo.description.host_information.ip_address.toString()}
      />
      <ExplorerListItem
        row
        divider
        label="Version"
        value={nodeInfo.description.build_information.build_version}
      />
      <ExplorerListItem row label="Active set Prob." value={activeSetProb} />
    </ExplorerCard>
  );
};
