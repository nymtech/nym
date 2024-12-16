import type NymNode from "@/app/api/types";
import { ClientOnly } from "../clientOnly/ClientOnly";
import NodeTable from "./NodeTable";
import getNymNodes from "./actions";

const mappedNymNodes = (nodes: NymNode[]) =>
  nodes.map((node) => {
    return {
      nodeId: node.node_id,
      bondInformation: node.bond_information,
      location: node.location,
      profitMarginPercentage:
        +node.rewarding_details.cost_params.profit_margin_percent * 100,
      description: node.description,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNymNodes>;
export type MappedNymNode = MappedNymNodes[0];

const NodeTableWithAction = async () => {
  try {
    const nodes = await getNymNodes();
    const data = mappedNymNodes(nodes);
    return (
      <ClientOnly>
        <NodeTable nodes={data} />
      </ClientOnly>
    );
  } catch (error) {
    console.error(error);
    return [];
  }
};

export default NodeTableWithAction;
