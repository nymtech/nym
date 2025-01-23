"use client";

import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import {
  CURRENT_EPOCH_REWARDS,
  DATA_OBSERVATORY_NODES_URL,
} from "@/app/api/urls";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  id: number; // Node ID
}

// Fetch functions
const fetchEpochRewards = async (): Promise<
  ExplorerData["currentEpochRewardsData"]
> => {
  const response = await fetch(CURRENT_EPOCH_REWARDS, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch epoch rewards");
  }

  return response.json();
};

const fetchNodeInfo = async (id: number): Promise<IObservatoryNode | null> => {
  const response = await fetch(DATA_OBSERVATORY_NODES_URL, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch observatory nodes");
  }

  const nodes: IObservatoryNode[] = await response.json();
  return nodes.find((node) => node.node_id === id) || null;
};

export const NodeMetricsCard = ({ id }: INodeMetricsCardProps) => {
  // Fetch epoch rewards
  const {
    data: epochRewardsData,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is fresh for 60 seconds
  });

  // Fetch node information
  const {
    data: nodeInfo,
    isLoading: isNodeLoading,
    isError: isNodeError,
  } = useQuery({
    queryKey: ["nodeInfo", id],
    queryFn: () => fetchNodeInfo(id),
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is fresh for 60 seconds
  });

  if (isEpochLoading || isNodeLoading) {
    return (
      <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
        <div>Loading...</div>
      </ExplorerCard>
    );
  }

  if (isEpochError || isNodeError || !nodeInfo || !epochRewardsData) {
    return (
      <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
        <div>Failed to load data</div>
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
