"use client";

import { fetchNodeDelegations } from "@/app/api";
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
  });

  return (
    <ExplorerCard label="Delegations" sx={{ height: "100%" }}>
      {isLoading && <div>Loading delegations...</div>}
      {isError && (
        <div>Failed to load delegations. Please try again later.</div>
      )}
      {!isLoading && !isError && <DelegationsTable delegations={delegations} />}
    </ExplorerCard>
  );
};

export default NodeDelegationsCard;
