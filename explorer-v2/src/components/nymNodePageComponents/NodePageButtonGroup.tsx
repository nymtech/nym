"use client";

import { fetchObservatoryNodes } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useQuery } from "@tanstack/react-query";

type Props = {
  paramId: string;
};

export default function NodePageButtonGroup({ paramId }: Props) {
  let nodeInfo: IObservatoryNode | undefined;

  const { data: nymNodes, isError } = useQuery({
    queryKey: ["nymNodes"],
    queryFn: fetchObservatoryNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (!nymNodes || isError) return null;

  // get node info based on wether it's dentity_key or node_id

  if (paramId.length > 10) {
    nodeInfo = nymNodes.find((node) => node.identity_key === paramId);
  } else {
    nodeInfo = nymNodes.find((node) => node.node_id === Number(paramId));
  }

  if (!nodeInfo) return null;

  if (nodeInfo.bonding_address)
    return (
      <ExplorerButtonGroup
        onPage="Nym Node"
        options={[
          {
            label: "Nym Node",
            isSelected: true,
            link: `/nym-node/${nodeInfo.node_id}`,
          },
          {
            label: "Account",
            isSelected: false,
            link: `/account/${nodeInfo.bonding_address}`,
          },
        ]}
      />
    );
}
