"use client";

import type { NS_NODE } from "@/app/api/types";
import { Skeleton, Stack, Typography, useTheme } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchNSApiNodes } from "../../app/api";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

type Props = {
  paramId: string;
};

export const BasicInfoCard = ({ paramId }: Props) => {
  let nodeInfo: NS_NODE | undefined;

  const {
    data: nsApiNodes = [],
    isLoading: isNSApiNodesLoading,
    isError: isNSApiNodesError,
  } = useQuery({
    queryKey: ["nsApiNodes"],
    queryFn: fetchNSApiNodes,
    staleTime: 10 * 60 * 1000, // 10 minutes
    refetchOnWindowFocus: false, // Prevents unnecessary refetching
    refetchOnReconnect: false,
    refetchOnMount: false,
  });

  const theme = useTheme();
  const isDarkMode = theme.palette.mode === "dark";

  if (isNSApiNodesLoading) {
    return (
      <ExplorerCard label="Basic info">
        <Skeleton variant="text" height={90} />
        <Skeleton variant="text" height={90} />
        <Skeleton variant="text" height={70} />
        <Skeleton variant="text" height={70} />
        <Skeleton variant="text" height={70} />
        <Skeleton variant="text" height={70} />
      </ExplorerCard>
    );
  }

  if (!nsApiNodes || isNSApiNodesError) {
    return (
      <ExplorerCard label="Basic info">
        <Typography
          variant="h3"
          sx={{ color: isDarkMode ? "base.white" : "pine.950" }}
        >
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

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

  const selfBond = nodeInfo.rewarding_details
    ? formatBigNum(Number(nodeInfo.rewarding_details.operator) / 1_000_000)
    : 0;
  const selfBondFormatted = `${selfBond} NYM`;

  return (
    <ExplorerCard label="Basic info" sx={{ height: "100%" }}>
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
              <Typography
                variant="body4"
                sx={{ wordWrap: "break-word", maxWidth: "90%" }}
              >
                {nodeInfo.bonding_address
                  ? nodeInfo.bonding_address
                  : "Node not bonded"}
              </Typography>
              {nodeInfo.bonding_address && (
                <CopyToClipboard text={nodeInfo.bonding_address} />
              )}
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
              <Typography
                variant="body4"
                sx={{ wordWrap: "break-word", maxWidth: "85%" }}
              >
                {nodeInfo.identity_key}
              </Typography>
              <CopyToClipboard text={nodeInfo.identity_key} />
            </Stack>
          }
        />

        {nodeInfo.rewarding_details && (
          <ExplorerListItem
            row
            divider
            label="Nr. of stakers"
            value={nodeInfo.rewarding_details.unique_delegations.toString()}
          />
        )}
        <ExplorerListItem row label="Self bonded" value={selfBondFormatted} />
      </Stack>
    </ExplorerCard>
  );
};
