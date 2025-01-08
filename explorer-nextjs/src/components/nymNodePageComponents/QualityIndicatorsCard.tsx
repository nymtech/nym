import type { IObservatoryNode, NodeDescription } from "@/app/api/types";
import { Chip, Stack } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import StarRating from "../starRating/StarRating";

interface IQualityIndicatorsCardProps {
  nodeDescription: NodeDescription;
  nodeInfo?: IObservatoryNode;
}

type NodeDescriptionNotNull = NonNullable<NodeDescription>;
type DelcaredRoleKey = keyof NodeDescriptionNotNull["declared_role"];
type RoleString = "Entry Node" | "Exit IPR Node" | "Exit NR Node" | "Mix Node";

const roleMapping: Record<DelcaredRoleKey, RoleString> = {
  entry: "Entry Node",
  exit_ipr: "Exit IPR Node",
  exit_nr: "Exit NR Node",
  mixnode: "Mix Node",
};

function getNodeRoles(
  declaredRoles: NodeDescriptionNotNull["declared_role"],
): RoleString[] {
  const activeRoles = Object.entries(declaredRoles)
    .filter(([, isActive]) => isActive)
    .map(([role]) => roleMapping[role as DelcaredRoleKey]);

  return activeRoles;
}

export const QualityIndicatorsCard = (props: IQualityIndicatorsCardProps) => {
  const { nodeDescription, nodeInfo } = props;

  if (!nodeDescription) {
    return null;
  }

  const nodeRoles = getNodeRoles(nodeDescription.declared_role);
  const NodeRoles = nodeRoles.map((role) => (
    <Stack key={role} direction="row" gap={1}>
      <Chip key={role} label={role} size="small" />
    </Stack>
  ));

  console.log("activeRoles :>> ", nodeDescription.declared_role);

  return (
    <ExplorerCard label="Quality indicatiors" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Role"
        value={
          <Stack direction="row" gap={1}>
            {NodeRoles}
          </Stack>
        }
      />
      <ExplorerListItem
        row
        divider
        label="Quality of service"
        value={<StarRating value={5} />}
      />
      <ExplorerListItem
        row
        divider
        label="Config score"
        value={<StarRating value={4} />}
      />
      <ExplorerListItem
        row
        divider
        label="Probe score"
        value={<StarRating value={5} />}
      />
    </ExplorerCard>
  );
};
