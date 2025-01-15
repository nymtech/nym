import getNymNodes from "@/actions/getNymNodes";
import type { IObservatoryNode } from "@/app/api/types";
import StakeTable from "./StakeTable";

const mappedNymNodes = (nodes: IObservatoryNode[]) =>
  nodes.map((node) => {
    return {
      name: node.self_description.moniker,
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

const StakeTableWithAction = async () => {
  try {
    const nodes = await getNymNodes();
    const data = mappedNymNodes(nodes);
    return <StakeTable nodes={data} />;
  } catch (error) {
    console.error(error);
    return null;
  }
};

export default StakeTableWithAction;
