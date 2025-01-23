import getNymNodes from "@/actions/getNymNodes";
import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import { CURRENT_EPOCH_REWARDS } from "@/app/api/urls";
import NodeTable from "./NodeTable";

async function fetchEpochRewards() {
  try {
    const response = await fetch(CURRENT_EPOCH_REWARDS, {
      headers: {
        Accept: "application/json",
        "Content-Type": "application/json; charset=utf-8",
      },
    });

    if (!response.ok) {
      throw new Error(`Failed to fetch epoch rewards: ${response.statusText}`);
    }

    return await response.json();
  } catch (error) {
    console.error("Error fetching epoch rewards:", error);
    throw new Error("Failed to fetch epoch rewards data");
  }
}

function getNodeSaturationPoint(
  totalStake: number,
  stakeSaturationPoint: string,
): number {
  const saturation = Number.parseFloat(stakeSaturationPoint);

  if (Number.isNaN(saturation) || saturation <= 0) {
    throw new Error("Invalid stake saturation point provided");
  }

  const ratio = (totalStake / saturation) * 100;

  return Number(ratio.toFixed());
}

const mappedNymNodes = (
  nodes: IObservatoryNode[],
  epochRewardsData: ExplorerData["currentEpochRewardsData"],
) =>
  nodes.map((node) => {
    const nodeSaturationPoint = getNodeSaturationPoint(
      node.total_stake,
      epochRewardsData.interval.stake_saturation_point,
    );

    return {
      name: node.self_description.moniker,
      nodeId: node.node_id,
      identity_key: node.identity_key,
      countryCode: node.description.auxiliary_details.location || null,
      countryName: node.description.auxiliary_details.location || null,
      profitMarginPercentage:
        +node.rewarding_details.cost_params.profit_margin_percent * 100,
      owner: node.bonding_address,
      stakeSaturation: nodeSaturationPoint,
      qualityOfService: +node.uptime * 100,
    };
  });

export type MappedNymNodes = ReturnType<typeof mappedNymNodes>;
export type MappedNymNode = MappedNymNodes[0];

const NodeTableWithAction = async () => {
  try {
    // Fetch the epoch rewards data
    const epochRewardsData: ExplorerData["currentEpochRewardsData"] =
      await fetchEpochRewards();

    // Fetch the Nym nodes
    const nodes = await getNymNodes();

    // Map the nodes with the rewards data
    const data = mappedNymNodes(nodes, epochRewardsData);

    return <NodeTable nodes={data} />;
  } catch (error) {
    console.error("Error in NodeTableWithAction:", error);
    return <div>Error loading data.</div>; // Render error fallback UI
  }
};

export default NodeTableWithAction;
