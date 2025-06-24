"use client";

import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { useQuery } from "@tanstack/react-query";
import { getBasePathByEnv } from "../../../envs/config";
import { Box } from "@mui/material";
import SectionHeading from "@/components/headings/SectionHeading";

type Props = {
  paramId: string;
};

export default function NodePageButtonGroup({ paramId }: Props) {
  let nodeInfo: NS_NODE | undefined;
  const { environment } = useEnvironment();
  const basePath = getBasePathByEnv(environment || "mainnet");

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
      <Box sx={{ display: "flex", justifyContent: "space-between" }}>
        <ExplorerButtonGroup
          onPage="Node"
          options={[
            {
              label: "Nym Node",
              isSelected: true,
              link: `${basePath}/nym-node/${nodeInfo.identity_key}`,
            },
            {
              label: "Account",
              isSelected: false,
              link: `${basePath}/account/${nodeInfo.bonding_address}`,
            },
          ]}
        />
      </Box>
    );
}
