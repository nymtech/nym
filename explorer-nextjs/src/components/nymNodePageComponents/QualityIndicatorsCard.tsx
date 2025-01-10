import type { IObservatoryNode, NodeDescription } from "@/app/api/types";
import { Chip, Stack } from "@mui/material";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import StarRating from "../starRating/StarRating";

interface IQualityIndicatorsCardProps {
  nodeInfo: IObservatoryNode;
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
  const { nodeInfo } = props;

  const nodeRoles = getNodeRoles(nodeInfo.description.declared_role);
  const NodeRoles = nodeRoles.map((role) => (
    <Stack key={role} direction="row" gap={1}>
      <Chip key={role} label={role} size="small" />
    </Stack>
  ));

  function calculateQualityOfServiceStars(quality: number): number {
    if (quality < 0.3) {
      return 1;
    }
    if (quality < 0.5) {
      return 2;
    }
    if (quality < 0.7) {
      return 3;
    }
    return 4;
  }
  const qualityOfServiceStars = nodeInfo?.uptime
    ? calculateQualityOfServiceStars(nodeInfo?.uptime)
    : 1;

  const nodeIsMixNodeOnly =
    NodeRoles.length === 1 && nodeRoles[0] === "Mix Node";

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
      {nodeIsMixNodeOnly && (
        <ExplorerListItem
          row
          divider
          label="Quality of service"
          value={<StarRating value={qualityOfServiceStars} />}
        />
      )}
      {!nodeIsMixNodeOnly && (
        <ExplorerListItem
          row
          divider
          label="Config score"
          value={<StarRating value={4} />}
        />
      )}
      {!nodeIsMixNodeOnly && (
        <ExplorerListItem
          row
          divider
          label="Probe score"
          value={<StarRating value={4} />}
        />
      )}
    </ExplorerCard>
  );
};
