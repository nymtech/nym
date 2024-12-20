import type { NodeDescription } from "@/app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  nodeDescription: NodeDescription;
  nodeId: number;
}

export const NodeMetricsCard = (props: INodeMetricsCardProps) => {
  const { nodeDescription, nodeId } = props;
  return (
    <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Node ID."
        value={nodeId.toString()}
      />
      <ExplorerListItem
        row
        divider
        label="Host"
        value={nodeDescription.host_information.ip_address.toString()}
      />
      <ExplorerListItem
        row
        divider
        label="Version"
        value={nodeDescription.build_information.build_version}
      />
      <ExplorerListItem row label="Active set Prob." value="High" />
    </ExplorerCard>
  );
};
