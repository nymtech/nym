"use client";

import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useQuery } from "@tanstack/react-query";

type Props = {
  address: string;
};

export default function AccountPageButtonGroup({ address }: Props) {
  const { data: nsApiNodes = [], isError: isNSApiNodesError } = useQuery({
    queryKey: ["nsApiNodes"],
    queryFn: fetchNSApiNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (!nsApiNodes || isNSApiNodesError) return null;

  const nymNode = nsApiNodes.find(
    (node: NS_NODE) => node.bonding_address === address
  );

  if (!nymNode) return null;

  if (nymNode.bonding_address)
    return (
      <ExplorerButtonGroup
        onPage="Account"
        options={[
          {
            label: "Nym Node",
            isSelected: false,
            link: nymNode
              ? `/nym-node/${nymNode.node_id}`
              : `/account/${address}/not-found`,
          },
          {
            label: "Account",
            isSelected: true,
            link: `/account/${address}`,
          },
        ]}
      />
    );
}
