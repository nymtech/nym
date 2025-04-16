"use client";

import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import DelegationsTable from "./DelegationsTable";

type Props = {
  paramId: string;
};

const NodeDelegationsCard = ({ paramId }: Props) => {
  let nodeInfo: NS_NODE | undefined;

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

  const id = nodeInfo.node_id;

  if (isNSApiNodesLoading) {
    return (
      <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isNSApiNodesError) {
    return (
      <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load delegations. Please try again later.
        </Typography>
      </ExplorerCard>
    );
  }

  return (
    <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
      <DelegationsTable id={id} />
    </ExplorerCard>
  );
};

export default NodeDelegationsCard;
