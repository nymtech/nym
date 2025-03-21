"use client";

import { fetchNodeDelegations } from "@/app/api";
import { Skeleton, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import ExplorerCard from "../cards/ExplorerCard";
import DelegationsTable from "./DelegationsTable";

interface NodeDelegationsCardProps {
  id: number; // Node ID
}

const NodeDelegationsCard = ({ id }: NodeDelegationsCardProps) => {
  const {
    data: delegations = [],
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nodeDelegations", id],
    queryFn: () => fetchNodeDelegations(id),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,

  });

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
      <DelegationsTable delegations={delegations} />
    </ExplorerCard>
  );
};

export default NodeDelegationsCard;
