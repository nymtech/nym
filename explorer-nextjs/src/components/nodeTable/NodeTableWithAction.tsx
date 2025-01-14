import getNymNodes from "@/actions/getNymNodes";
import type { IObservatoryNode } from "@/app/api/types";
import NodeTable from "./NodeTable";

const mappedNymNodes = (nodes: IObservatoryNode[]) =>
  nodes.map((node) => {
    return {
      nodeId: node.node_id,
      identity_key: node.identity_key,
      countryCode: node.description.auxiliary_details.location || null,
      countryName: node.description.auxiliary_details.location || null,
      profitMarginPercentage:
        +node.rewarding_details.cost_params.profit_margin_percent * 100,
      owner: node.bonding_address,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNymNodes>;
export type MappedNymNode = MappedNymNodes[0];

const NodeTableWithAction = async () => {
  try {
    const nodes = await getNymNodes();
    const data = mappedNymNodes(nodes);
    return <NodeTable nodes={data} />;
  } catch (error) {
    console.error(error);
    return [];
  }
};

export default NodeTableWithAction;
