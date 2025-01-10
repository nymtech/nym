import type { IObservatoryNode, RewardingDetails } from "@/app/api/types";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

interface IBasicInfoCardProps {
  rewardDetails: RewardingDetails;
  nodeInfo: IObservatoryNode;
}

export const BasicInfoCard = (props: IBasicInfoCardProps) => {
  const { rewardDetails, nodeInfo } = props;

  const timeBonded = nodeInfo
    ? format(
        new Date(nodeInfo.description.build_information.build_timestamp),
        "dd/MM/yyyy",
      )
    : "-";

  const selfBond = formatBigNum(Number(rewardDetails.operator) / 1_000_000);
  const selfBondFormated = `${selfBond} NYM`;
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
          label="Nr. of stakes"
          value={rewardDetails.unique_delegations.toString()}
        />
        <ExplorerListItem row label="Self bonded" value={selfBondFormated} />
      </Stack>
    </ExplorerCard>
  );
};
