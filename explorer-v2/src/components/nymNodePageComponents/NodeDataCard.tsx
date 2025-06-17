"use client";

import type { NS_NODE } from "@/app/api/types";
import { Skeleton, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { fetchNSApiNodes } from "../../app/api";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import { useEnvironment } from "@/providers/EnvironmentProvider";

type Props = {
  paramId: string;
};

export const NodeDataCard = ({ paramId }: Props) => {
  let nodeInfo: NS_NODE | undefined;
  const { environment } = useEnvironment();

  // Fetch node information
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
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isNSApiNodesError || !nsApiNodes) {
    return (
      <ExplorerCard label="Nym node data" sx={{ height: "100%" }}>
        <Typography
          variant="h5"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  // get node info based on wether it's dentity_key or node_id

  if (paramId.length > 10) {
    nodeInfo = nsApiNodes.find(
      (node: NS_NODE) => node.identity_key === paramId
    );
  } else {
    nodeInfo = nsApiNodes.find(
      (node: NS_NODE) => node.node_id === Number(paramId)
    );
  }

  if (!nodeInfo) return null;

  const softwareUpdateTime = nodeInfo.self_description
    ? format(
        new Date(nodeInfo.self_description.build_information.build_timestamp),
        "dd/MM/yyyy"
      )
    : "N/A";

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
        value={
          nodeInfo.self_description
            ? nodeInfo.self_description.host_information.ip_address.toString()
            : "N/A"
        }
      />
      <ExplorerListItem
        row
        divider
        label="Version"
        value={
          nodeInfo.self_description
            ? nodeInfo.self_description.build_information.build_version
            : "N/A"
        }
      />
      <ExplorerListItem
        row
        label="Last software update"
        value={softwareUpdateTime}
      />
    </ExplorerCard>
  );
};
