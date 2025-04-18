"use client";

import { formatBigNum } from "@/utils/formatBigNumbers";
import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchEpochRewards, fetchObservatoryNodes } from "../../app/api";
import type { IObservatoryNode, RewardingDetails } from "../../app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

type Props = {
  paramId: string;
};

export const NodeParametersCard = ({ paramId }: Props) => {
  let nodeInfo: IObservatoryNode | undefined;

  // Fetch epoch rewards
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

  // Fetch node information
  const {
    data: nymNodes,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (isEpochLoading || isLoading) {
    return (
      <ExplorerCard label="Node parameters" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isEpochError || isError || !nymNodes || !epochRewardsData) {
    return (
      <ExplorerCard label="Node parameters" sx={{ height: "100%" }}>
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }
  // get node info based on wether it's dentity_key or node_id

  if (paramId.length > 10) {
    nodeInfo = nymNodes.find((node) => node.identity_key === paramId);
  } else {
    nodeInfo = nymNodes.find((node) => node.node_id === Number(paramId));
  }

  if (!nodeInfo) return null;

  const totalStake = formatBigNum(Number(nodeInfo.total_stake) / 1_000_000);
  const totalStakeFormatted = `${totalStake} NYM`;

  // Extract reward details
  const rewardDetails: RewardingDetails = nodeInfo.rewarding_details;

  const profitMarginPercent =
    Number(rewardDetails.cost_params.profit_margin_percent) * 100;
  const profitMarginPercentFormated = `${profitMarginPercent}%`;

  const operatingCosts =
    Number(rewardDetails.cost_params.interval_operating_cost.amount) /
    1_000_000;
  const operatingCostsFormated = `${operatingCosts.toString()} NYM`;

  const getNodeSaturationPoint = (
    totalStake: number,
    stakeSaturationPoint: string,
  ): string => {
    const saturation = Number.parseFloat(stakeSaturationPoint);

    if (Number.isNaN(saturation) || saturation <= 0) {
      throw new Error("Invalid stake saturation point provided");
    }

    const ratio = (totalStake / saturation) * 100;

    return `${ratio.toFixed()}%`;
  };

  const nodeSaturationPoint = getNodeSaturationPoint(
    nodeInfo.total_stake,
    epochRewardsData.interval.stake_saturation_point,
  );

  return (
    <ExplorerCard label="Node parameters" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Total stake"
        value={totalStakeFormatted}
      />
      {/* <ExplorerListItem
        row
        divider
        label="Operator rew."
        value={operatorRewardsFormated}
      /> */}
      <ExplorerListItem
        row
        divider
        label="Profit margin rew."
        value={profitMarginPercentFormated}
      />
      <ExplorerListItem
        row
        divider
        label="Operating cost"
        value={operatingCostsFormated}
      />
      <ExplorerListItem
        row
        label="Node stake saturation"
        value={nodeSaturationPoint}
      />
    </ExplorerCard>
  );
};
