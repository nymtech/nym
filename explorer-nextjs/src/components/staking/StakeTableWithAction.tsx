"use client";

import { useQuery } from "@tanstack/react-query";
import getNymNodes from "../../actions/getNymNodes";
import { fetchEpochRewards } from "../../app/api";
import type { ExplorerData, IObservatoryNode } from "../../app/api/types";
import StakeTable from "./StakeTable";

// Utility function to calculate node saturation point
function getNodeSaturationPoint(
  totalStake: number,
  stakeSaturationPoint: string,
): number {
  const saturation = Number.parseFloat(stakeSaturationPoint);

  if (Number.isNaN(saturation) || saturation <= 0) {
    throw new Error("Invalid stake saturation point provided");
  }

  const ratio = (totalStake / saturation) * 100;
  return Number(ratio.toFixed());
}

// Map nodes with rewards data
const mappedNymNodes = (
  nodes: IObservatoryNode[],
  epochRewardsData: ExplorerData["currentEpochRewardsData"],
) =>
  nodes.map((node) => {
    const nodeSaturationPoint = getNodeSaturationPoint(
      node.total_stake,
      epochRewardsData.interval.stake_saturation_point,
    );

    return {
      name: node.self_description.moniker,
      nodeId: node.node_id,
      identity_key: node.identity_key,
      countryCode: node.description.auxiliary_details.location || null,
      countryName: node.description.auxiliary_details.location || null,
      profitMarginPercentage:
        +node.rewarding_details.cost_params.profit_margin_percent * 100,
      owner: node.bonding_address,
      stakeSaturation: nodeSaturationPoint || 0,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNymNodes>;
export type MappedNymNode = MappedNymNodes[0];

const StakeTableWithAction = () => {
  // Use React Query to fetch epoch rewards
  const {
    data: epochRewardsData,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
    staleTime: 60000, // Data is fresh for 60 seconds
    refetchInterval: 60000, // Refetch every 60 seconds
  });

  // Use React Query to fetch Nym nodes
  const {
    data: nymNodes = [],
    isLoading: isNodesLoading,
    isError: isNodesError,
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: getNymNodes,
    staleTime: 60000,
    refetchInterval: 60000,
  });

  // Handle loading state
  if (isEpochLoading || isNodesLoading) {
    return <div>Loading stake table...</div>;
  }

  // Handle error state
  if (isEpochError || isNodesError) {
    return <div>Error loading stake table data. Please try again later.</div>;
  }

  // Map nodes with rewards data
  const data = mappedNymNodes(nymNodes, epochRewardsData);

  return <StakeTable nodes={data} />;
};

export default StakeTableWithAction;
