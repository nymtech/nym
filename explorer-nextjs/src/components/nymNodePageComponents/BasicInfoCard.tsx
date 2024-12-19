import type {
  BondInformation,
  NodeDescription,
  RewardingDetails,
} from "@/app/api/types";
import { formatBigNum } from "@/utils/formatBigNumbers";
import { Stack, Typography } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

interface IBasicInfoCardProps {
  bondInfo: BondInformation;
  nodeDescription: NodeDescription;
  rewardDetails: RewardingDetails;
}

export const BasicInfoCard = (props: IBasicInfoCardProps) => {
  const { bondInfo, nodeDescription, rewardDetails } = props;

  const timeBonded = format(
    new Date(nodeDescription.build_information.build_timestamp),
    "dd/MM/yyyy",
  );

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
              <Typography variant="body4">{bondInfo.owner}</Typography>
              <CopyToClipboard text={bondInfo.owner} />
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
              <Typography variant="body4">
                {bondInfo.node.identity_key}
              </Typography>
              <CopyToClipboard text={bondInfo.node.identity_key} />
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
