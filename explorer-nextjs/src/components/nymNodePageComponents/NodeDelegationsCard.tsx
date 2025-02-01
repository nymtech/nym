"use client";

import { useQuery } from "@tanstack/react-query";
import type { NodeRewardDetails } from "../../app/api/types";
import { DATA_OBSERVATORY_NODES_URL } from "../../app/api/urls";
import ExplorerCard from "../cards/ExplorerCard";
import DelegationsTable from "./DelegationsTable";

interface NodeDelegationsCardProps {
  id: number; // Node ID
}

// Fetch delegations dynamically based on ID
const fetchNodeDelegations = async (
  id: number,
): Promise<NodeRewardDetails[]> => {
  const response = await fetch(
    `${DATA_OBSERVATORY_NODES_URL}/${id}/delegations`,
    {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
      next: { revalidate: 60 },
    },
  );

  if (!response.ok) {
    throw new Error("Failed to fetch delegations");
  }

  return response.json();
};

const NodeDelegationsCard = ({ id }: NodeDelegationsCardProps) => {
  // Use React Query to fetch delegations
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
