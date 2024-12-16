import type { IBondInfo, INodeDescription } from "@/app/api";
import { Stack } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
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
          value={bondInfo.bond_information.owner}
        />
        <ExplorerListItem
          divider
          label="Identity Key"
          value={bondInfo.bond_information.node.identity_key}
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
