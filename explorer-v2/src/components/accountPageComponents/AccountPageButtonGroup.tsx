"use client";

import { fetchObservatoryNodes } from "@/app/api";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useQuery } from "@tanstack/react-query";

type Props = {
  address: string;
};

export default function AccountPageButtonGroup({ address }: Props) {
  const { data: nymNodes, isError } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (!nymNodes || isError) return null;

  const nymNode = nymNodes.find((node) => node.bonding_address === address);

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
