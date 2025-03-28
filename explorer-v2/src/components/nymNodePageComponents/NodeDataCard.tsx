"use client";

import { useQuery } from "@tanstack/react-query";
import { fetchEpochRewards, fetchObservatoryNodes } from "../../app/api";
import { Skeleton, Typography } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import { IObservatoryNode } from "@/app/api/types";

type Props = {
  paramId: string;
};

export const NodeDataCard = ({ paramId }: Props) => {
  let nodeInfo: IObservatoryNode | undefined

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
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isEpochError || isError || !nymNodes || !epochRewardsData) {
    return (
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
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
