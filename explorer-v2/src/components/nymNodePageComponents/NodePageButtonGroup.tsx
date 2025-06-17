"use client";

import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { useQuery } from "@tanstack/react-query";

type Props = {
  paramId: string;
};

export default function NodePageButtonGroup({ paramId }: Props) {
  let nodeInfo: NS_NODE | undefined;
  const { environment } = useEnvironment();

  const { data: nsApiNodes = [], isError: isNSApiNodesError } = useQuery({
    queryKey: ["nsApiNodes", environment],
    queryFn: () => fetchNSApiNodes(environment),
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  if (!nsApiNodes || isNSApiNodesError) return null;

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
