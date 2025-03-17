"use client";

import { Skeleton, Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { fetchNodeInfo } from "../../app/api";
import { formatBigNum } from "../../utils/formatBigNumbers";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

interface IBasicInfoCardProps {
  id: number;
}

export const BasicInfoCard = ({ id }: IBasicInfoCardProps) => {
  const {
    data: nodeInfo,
    isLoading,
    isError,
  } = useQuery({
    queryKey: ["nodeInfo", id],
    queryFn: () => fetchNodeInfo(id),
  });

  if (isLoading) {
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

  if (isError || !nodeInfo) {
    return (
      <ExplorerCard label="Basic info">
        <Typography variant="h3" sx={{ color: "pine.950" }}>
          Failed to load node data.
        </Typography>
      </ExplorerCard>
    );
  }

  const selfBond = formatBigNum(
    Number(nodeInfo.rewarding_details.operator) / 1_000_000,
  );
  const selfBondFormatted = `${selfBond} NYM`;

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
              <Typography
                variant="body4"
                sx={{ wordWrap: "break-word", maxWidth: "90%" }}
              >
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

        <ExplorerListItem
          row
          divider
          label="Nr. of stakers"
          value={nodeInfo.rewarding_details.unique_delegations.toString()}
        />
        <ExplorerListItem row label="Self bonded" value={selfBondFormatted} />
      </Stack>
    </ExplorerCard>
  );
};
