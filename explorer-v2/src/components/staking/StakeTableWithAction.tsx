"use client";

import { Card, CardContent, Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import DOMPurify from "isomorphic-dompurify";
import { fetchEpochRewards, fetchNSApiNodes } from "../../app/api";
import type { ExplorerData, NS_NODE } from "../../app/api/types";
import { countryName } from "../../utils/countryName";
import StakeTable from "./StakeTable";

// Utility function to calculate node saturation point
function getNodeSaturationPoint(
  totalStake: number,
  stakeSaturationPoint: string
): number {
  const saturation = Number.parseFloat(stakeSaturationPoint);

  if (Number.isNaN(saturation) || saturation <= 0) {
    throw new Error("Invalid stake saturation point provided");
  }

  const ratio = (totalStake / saturation) * 100;
  return Number.parseFloat(ratio.toFixed());
}

// Map nodes with rewards data
const mappedNSApiNodes = (
  nodes: NS_NODE[],
  epochRewardsData: ExplorerData["currentEpochRewardsData"]
) =>
  nodes.map((node) => {
    const nodeSaturationPoint = getNodeSaturationPoint(
      +node.total_stake,
      epochRewardsData.interval.stake_saturation_point
    );

    const cleanMoniker = DOMPurify.sanitize(node.description.moniker).replace(
      /&amp;/g,
      "&"
    );

    return {
      name: cleanMoniker,
      nodeId: node.node_id,
      identity_key: node.identity_key,
      countryCode: node.geoip?.country || null,
      countryName: countryName(node.geoip?.country || null) || null,
      profitMarginPercentage: node.rewarding_details
        ? +node.rewarding_details.cost_params.profit_margin_percent * 100
        : 0,
      owner: node.bonding_address,
      stakeSaturation: +nodeSaturationPoint || 0,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNSApiNodes>;
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
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Use React Query to fetch Nym nodes
  const {
    data: nsApiNodes = [],
    isLoading: isNSApiNodesLoading,
    isError: isNSApiNodesError,
  } = useQuery({
    queryKey: ["nsApiNodes"],
    queryFn: fetchNSApiNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  // Handle loading state
  if (isEpochLoading || isNSApiNodesLoading) {
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
  if (isEpochError || isNSApiNodesError) {
    return (
      <Stack direction="row" spacing={1}>
        <Typography variant="h5" sx={{ color: "pine.600", letterSpacing: 0.7 }}>
          Error loading stake table data. Please try again later.
        </Typography>
      </Stack>
    );
  }

  // Map nodes with rewards data

  if (!epochRewardsData) {
    return null;
  }

  const nsApiNodesData = mappedNSApiNodes(nsApiNodes || [], epochRewardsData);

  return <StakeTable nodes={nsApiNodesData} />;
};

export default StakeTableWithAction;
