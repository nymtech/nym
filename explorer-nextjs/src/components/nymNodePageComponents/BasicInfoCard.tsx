import type { IBondInfo, INodeDescription } from "@/app/api";
import { Stack, Typography } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import CopyToClipboard from "../copyToClipboard/CopyToClipboard";
import ExplorerListItem from "../list/ListItem";

interface IBasicInfoCardProps {
  bondInfo: IBondInfo;
  nodeDescription: INodeDescription;
}

export const BasicInfoCard = (props: IBasicInfoCardProps) => {
  const { bondInfo, nodeDescription } = props;

  const timeBonded = format(
    new Date(nodeDescription.description.build_information.build_timestamp),
    "dd/MM/yyyy",
  );

  const selfBond = Number(bondInfo.rewarding_details.unit_delegation) / 1000000;
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
                {bondInfo.bond_information.owner}
              </Typography>
              <CopyToClipboard text={bondInfo.bond_information.owner} />
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
                {bondInfo.bond_information.node.identity_key}
              </Typography>
              <CopyToClipboard
                text={bondInfo.bond_information.node.identity_key}
              />
            </Stack>
          }
        />
        <ExplorerListItem row divider label="Node bonded" value={timeBonded} />
        <ExplorerListItem
          row
          divider
          label="Nr. of stakes"
          value={bondInfo.rewarding_details.unique_delegations.toString()}
        />
        <ExplorerListItem row label="Self bonded" value={selfBondFormated} />
      </Stack>
    </ExplorerCard>
  );
};
