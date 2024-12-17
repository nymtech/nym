import type { INodeDescription } from "@/app/api";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  nodeDescription: INodeDescription;
}

export const NodeMetricsCard = (props: INodeMetricsCardProps) => {
  const { nodeDescription } = props;
  return (
    <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Node ID."
        value={nodeDescription.node_id.toString()}
      />
      <ExplorerListItem
        row
        divider
        label="Host"
        value={nodeDescription.description.host_information.ip_address.toString()}
      />
      <ExplorerListItem row divider label="Version" value="1.1.1.1" />
      <ExplorerListItem row label="Active set Prob." value="High" />
    </ExplorerCard>
  );
};
