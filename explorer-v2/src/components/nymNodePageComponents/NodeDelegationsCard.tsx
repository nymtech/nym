"use client";

import {  fetchObservatoryNodes } from "@/app/api";
import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import DelegationsTable from "./DelegationsTable";
import { IObservatoryNode } from "@/app/api/types";

type Props = {
  paramId: string;
};

const NodeDelegationsCard = ({ paramId }: Props) => {
  let nodeInfo: IObservatoryNode | undefined

  const {
    data: nymNodes,
    isError,
    isLoading
  } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });



  if (paramId.length > 10) {
    nodeInfo = nymNodes?.find((node) => node.identity_key === paramId);

  } else {
    nodeInfo = nymNodes?.find((node) => node.node_id === Number(paramId));
  }

  if (!nodeInfo) return null;

  const id = nodeInfo.node_id

  if (isLoading) {
    return (
      <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
        <Skeleton variant="text" height={50} />
      </ExplorerCard>
    );
  }

  if (isError) {
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
