import type { INodeDescription } from "@/app/api";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";
import StarRating from "../starRating/StarRating";

interface IQualityIndicatorsCardProps {
  nodeDescription: INodeDescription;
}

interface IDeclaredRoles {
  declared_role: {
    entry: boolean;
    exit_ipr: boolean;
    exit_nr: boolean;
    mixnode: boolean;
  };
}

function getNodeRoles(rolesObject: IDeclaredRoles): string {
  const roleMapping: { [key: string]: string } = {
    entry: "Entry Node",
    exit_ipr: "Exit IPR Node",
    exit_nr: "Exit NR Node",
    mixnode: "Mix Node",
  };

  const { declared_role } = rolesObject;

  const activeRoles = Object.entries(declared_role)
    .filter(([_, value]) => value) // Filter keys where value is true
    .map(([key]) => roleMapping[key]) // Map keys to their corresponding strings
    .join(", "); // Join with commas

  return activeRoles;
}

export const QualityIndicatorsCard = (props: IQualityIndicatorsCardProps) => {
  const { nodeDescription } = props;

  const nodeRoles = getNodeRoles({
    declared_role: nodeDescription.description.declared_role,
  });

  return (
    <ExplorerCard label="Quality indicatiors" sx={{ height: "100%" }}>
      <ExplorerListItem row divider label="Role" value={nodeRoles} />
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
