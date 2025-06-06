"use client";

import { Card, CardContent, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import DOMPurify from "isomorphic-dompurify";
import { fetchEpochRewards, fetchObservatoryNodes } from "../../app/api";
import type { ExplorerData, IObservatoryNode } from "../../app/api/types";
import { countryName } from "../../utils/countryName";
import NodeTable from "./NodeTable";

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

    const cleanMoniker = DOMPurify.sanitize(
      node.self_description.moniker,
    ).replace(/&amp;/g, "&");

    return {
      name: cleanMoniker,
      nodeId: node.node_id,
      identity_key: node.identity_key,
      countryCode: node.description.auxiliary_details.location || null,
      countryName:
        countryName(node.description.auxiliary_details.location) || null,
      profitMarginPercentage:
        +node.rewarding_details.cost_params.profit_margin_percent * 100,
      owner: node.bonding_address,
      stakeSaturation: nodeSaturationPoint,
      qualityOfService: +node.uptime * 100,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNymNodes>;
export type MappedNymNode = MappedNymNodes[0];

const NodeTableWithAction = () => {
  // Use React Query to fetch epoch rewards
  const {
    data: epochRewardsData,
    isLoading: isEpochLoading,
    isError: isEpochError,
  } = useQuery({
    queryKey: ["epochRewards"],
    queryFn: fetchEpochRewards,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Use React Query to fetch Nym nodes
  const {
    data: nymNodes = [],
    isLoading: isNodesLoading,
    isError: isNodesError,
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Handle loading state
  if (isEpochLoading || isNodesLoading) {
    return (
      <Card sx={{ height: "100%", mt: 5 }}>
        <CardContent>
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
          <Skeleton variant="text" height={100} />
        </CardContent>
      </Card>
    );
  }

  // Handle error state
  if (isEpochError || isNodesError) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Error loading data. Please try again later.
        </Typography>
      </Stack>
    );
  }

  // Map nodes with rewards data

  if (!epochRewardsData) {
    return null;
  }

  const data = mappedNymNodes(nymNodes || [], epochRewardsData);

  return <NodeTable nodes={data} />;
};

export default NodeTableWithAction;
