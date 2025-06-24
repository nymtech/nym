"use client";

import { fetchNSApiNodes } from "@/app/api";
import type { NS_NODE } from "@/app/api/types";
import ExplorerButtonGroup from "@/components/toggleButton/ToggleButton";
import { useEnvironment } from "@/providers/EnvironmentProvider";
import { useQuery } from "@tanstack/react-query";
import { getBasePathByEnv } from "../../../envs/config";
import { Box } from "@mui/material";

type Props = {
  address: string;
};

export default function AccountPageButtonGroup({ address }: Props) {
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

  const nymNode = nsApiNodes.find(
    (node: NS_NODE) => node.bonding_address === address
  );

  if (!nymNode) return null;

  if (nymNode.bonding_address)
    return (
      <Box sx={{ display: "flex", justifyContent: "space-between" }}>
        <ExplorerButtonGroup
          onPage="Account"
          options={[
            {
              label: "Nym Node",
              isSelected: false,
              link: nymNode
                ? `${basePath}/nym-node/${nymNode.node_id}`
                : `${basePath}/account/${address}/not-found`,
            },
            {
              label: "Account",
              isSelected: true,
              link: `${basePath}/account/${address}`,
            },
          ]}
        />
      </Box>
    );
}
