import type { ExplorerData } from "@/app/api";
import type { IObservatoryNode, RewardingDetails } from "@/app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeRewardsCardProps {
  rewardDetails: RewardingDetails;
  nodeInfo?: IObservatoryNode;
  epochRewardsData: ExplorerData["currentEpochRewardsData"];
}

export const NodeRewardsCard = (props: INodeRewardsCardProps) => {
  const { rewardDetails, epochRewardsData, nodeInfo } = props;

  const operatorRewards = Number(rewardDetails.operator) / 1000000;
  const operatorRewardsFormated = `${operatorRewards} NYM`;

  const profitMarginPercent =
    Number(rewardDetails.cost_params.profit_margin_percent) * 100;

  const profitMarginPercentFormated = `${profitMarginPercent}%`;

  const operatingCosts =
    Number(rewardDetails.cost_params.interval_operating_cost.amount) / 1000000;
  const operatingCostsFormated = `${operatingCosts.toString()} NYM`;

  function getNodeSaturationPoint(
    totalStake: number,
    stakeSaturationPoint: string,
  ): string {
    const saturation = Number.parseFloat(stakeSaturationPoint);

    if (Number.isNaN(saturation) || saturation <= 0) {
      throw new Error("Invalid stake saturation point provided");
    }

    const ratio = (totalStake / saturation) * 100;

    return `${ratio.toFixed()}%`;
  }

  const nodeSaturationPoint =
    nodeInfo && epochRewardsData
      ? getNodeSaturationPoint(
          nodeInfo.total_stake,
          epochRewardsData.interval.stake_saturation_point,
        )
      : "N/A";

  return (
    <ExplorerCard label="Node rewards(last epoch/hour)" sx={{ height: "100%" }}>
      {/* <ExplorerListItem
        row
        divider
        label="Total rew."
        value={totalRewardsFormated}
      /> */}
      <ExplorerListItem
        row
        divider
        label="Operator rew."
        value={operatorRewardsFormated}
      />
      {/* <ExplorerListItem
        row
        divider
        label="Staker rew."
        value={stakerRewardsFormated}
      /> */}
      <ExplorerListItem
        row
        divider
        label="Profit margin rew."
        value={profitMarginPercentFormated}
      />
      <ExplorerListItem
        row
        divider
        label="Operating cost"
        value={operatingCostsFormated}
      />
      <ExplorerListItem
        row
        label="Saturation point"
        value={nodeSaturationPoint}
      />
    </ExplorerCard>
  );
};
