import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode } from "@/app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeMetricsCardProps {
  nodeInfo: IObservatoryNode;
  epochRewardsData: ExplorerData["currentEpochRewardsData"];
}

export const NodeMetricsCard = async (props: INodeMetricsCardProps) => {
  const { nodeInfo, epochRewardsData } = props;

  function getActiveSetProbability(
    totalStake: number,
    stakeSaturationPoint: string,
  ): string {
    const saturation = Number.parseFloat(stakeSaturationPoint);

    if (Number.isNaN(saturation) || saturation <= 0) {
      throw new Error("Invalid stake saturation point provided");
    }

    const ratio = (totalStake / saturation) * 100;

    if (ratio > 70) {
      return "High";
    }
    if (ratio >= 40 && ratio <= 70) {
      return "Medium";
    }
    return "Low";
  }

  const activeSetProb =
    nodeInfo && epochRewardsData
      ? getActiveSetProbability(
          nodeInfo.total_stake,
          epochRewardsData.interval.stake_saturation_point,
        )
      : "N/A";

  return (
    <ExplorerCard label="Nym node metrics" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Node ID."
        value={nodeInfo.node_id.toString()}
      />
      <>
        <ExplorerListItem
          row
          divider
          label="Host"
          value={nodeInfo.description.host_information.ip_address.toString()}
        />
        <ExplorerListItem
          row
          divider
          label="Version"
          value={nodeInfo.description.build_information.build_version}
        />
      </>
      {epochRewardsData && (
        <ExplorerListItem row label="Active set Prob." value={activeSetProb} />
      )}
    </ExplorerCard>
  );
};
