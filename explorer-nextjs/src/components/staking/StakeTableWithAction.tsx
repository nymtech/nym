import getNymNodes from "@/actions/getNymNodes";
import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import { CURRENT_EPOCH_REWARDS } from "@/app/api/urls";
import StakeTable from "./StakeTable";

const epochRewards = await fetch(CURRENT_EPOCH_REWARDS, {
  headers: {
    Accept: "application/json",
    "Content-Type": "application/json; charset=utf-8",
  },
});
const epochRewardsData: ExplorerData["currentEpochRewardsData"] =
  await epochRewards.json();

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

const mappedNymNodes = (nodes: IObservatoryNode[]) =>
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
      stakeSaturation: nodeSaturationPoint || 0,
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
