"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchEpochRewards, fetchNodeInfo } from "../../app/api";

import { Skeleton, Typography } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  id: number; // Node ID
}

export const NodeDataCard = ({ id }: INodeMetricsCardProps) => {
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
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isEpochError || isNodeError || !nodeInfo || !epochRewardsData) {
    return (
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  const softwareUpdateTime = format(
    new Date(nodeInfo.description.build_information.build_timestamp),
    "dd/MM/yyyy",
  );

  return (
    <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
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
      <ExplorerListItem
        row
        label="Last software update"
        value={softwareUpdateTime}
      />
    </ExplorerCard>
  );
};
