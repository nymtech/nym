"use client";

import type { IObservatoryNode } from "@/app/api/types";
import { DATA_OBSERVATORY_NODES_URL } from "@/app/api/urls";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

interface IBasicInfoCardProps {
  id: number; // Node ID
}

// Fetch function to get the node data
const fetchNodeInfo = async (id: number): Promise<IObservatoryNode | null> => {
  const response = await fetch(DATA_OBSERVATORY_NODES_URL, {
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json; charset=utf-8",
    },
    next: { revalidate: 60 },
  });

  if (!response.ok) {
    throw new Error("Failed to fetch observatory nodes");
  }

  const observatoryNymNodes: IObservatoryNode[] = await response.json();

  return observatoryNymNodes.find((node) => node.node_id === id) || null;
};

export const BasicInfoCard = ({ id }: IBasicInfoCardProps) => {
  // Use React Query to fetch the node info
  const {
    data: nodeInfo,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nodeInfo", id], // Unique query key based on the node ID
    queryFn: () => fetchNodeInfo(id), // Fetch function
    refetchInterval: 60000, // Refetch every 60 seconds
    staleTime: 60000, // Data is considered fresh for 60 seconds
  });

  // Loading state
  if (isLoading) {
    return (
      <ExplorerCard label="Basic info">
        <Typography>Loading...</Typography>
      </ExplorerCard>
    );
  }

  // Error state
  if (isError || !nodeInfo) {
    return (
      <ExplorerCard label="Basic info">
        <Typography>Failed to load node information.</Typography>
      </ExplorerCard>
    );
  }

  // Derived data from nodeInfo
  const timeBonded = format(
    new Date(nodeInfo.description.build_information.build_timestamp),
    "dd/MM/yyyy",
  );

  const selfBond = formatBigNum(
    Number(nodeInfo.rewarding_details.operator) / 1_000_000,
  );
  const selfBondFormatted = `${selfBond} NYM`;

  const totalStake = formatBigNum(Number(nodeInfo.total_stake) / 1_000_000);
  const totalStakeFormatted = `${totalStake} NYM`;

  return (
    <ExplorerCard label="Basic info">
      <Stack gap={1}>
        <ExplorerListItem
          divider
          label="NYM Address"
          value={
            <Stack
              direction="row"
              gap={0.1}
              alignItems="center"
              justifyContent="space-between"
              width="100%"
            >
              <Typography variant="body4">
                {nodeInfo.bonding_address}
              </Typography>
              <CopyToClipboard text={nodeInfo.bonding_address} />
            </Stack>
          }
        />
        <ExplorerListItem
          divider
          label="Identity Key"
          value={
            <Stack
              direction="row"
              gap={0.1}
              alignItems="center"
              justifyContent="space-between"
              width="100%"
            >
              <Typography variant="body4">{nodeInfo.identity_key}</Typography>
              <CopyToClipboard text={nodeInfo.identity_key} />
            </Stack>
          }
        />
        <ExplorerListItem row divider label="Node bonded" value={timeBonded} />
        <ExplorerListItem
          row
          divider
          label="Nr. of stakers"
          value={nodeInfo.rewarding_details.unique_delegations.toString()}
        />
        <ExplorerListItem
          row
          divider
          label="Self bonded"
          value={selfBondFormatted}
        />
        <ExplorerListItem row label="Total stake" value={totalStakeFormatted} />
      </Stack>
    </ExplorerCard>
  );
};
