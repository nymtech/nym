"use client";

import { Stack, Typography } from "@mui/material";
import { useQuery } from "@tanstack/react-query";
import { format } from "date-fns";
import { fetchAccountBalance, fetchNodeInfo } from "../../app/api";
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

  const address = "n1z0msxu8c098umdhnthpr2ac3ck2n3an97dm8pn";

  const { data: accountInfo } = useQuery({
    queryKey: ["accountBalance", address],
    queryFn: () => fetchAccountBalance(address),
    enabled: !!address,
  });

  console.log(" accountInfo basic info card:>> ", accountInfo);

  if (isLoading) {
    return (
      <ExplorerCard label="Basic info">
        <Typography>Loading...</Typography>
      </ExplorerCard>
    );
  }

  if (isError || !nodeInfo) {
    return (
      <ExplorerCard label="Basic info">
        <Typography>Failed to load node information.</Typography>
      </ExplorerCard>
    );
  }

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
