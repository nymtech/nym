import type { IObservatoryNode, RewardingDetails } from "@/app/api/types";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeRewardsCardProps {
  rewardDetails: RewardingDetails;
  nodeInfo?: IObservatoryNode;
}

export const NodeRewardsCard = (props: INodeRewardsCardProps) => {
  const { rewardDetails } = props;

  // const totalRewards = Number(rewardDetails.total_unit_reward) / 1000000;
  // const totalRewardsFormated = `${totalRewards} NYM`;

  const operatorRewards = Number(rewardDetails.operator) / 1000000;
  const operatorRewardsFormated = `${operatorRewards} NYM`;

  // const stakerRewards = Number(rewardDetails.delegates) / 1000000;
  // const stakerRewardsFormated = `${stakerRewards} NYM`;

  const profitMarginPercent =
    Number(rewardDetails.cost_params.profit_margin_percent) * 100;

  const profitMarginPercentFormated = `${profitMarginPercent}%`;

  const operatingCosts =
    Number(rewardDetails.cost_params.interval_operating_cost.amount) / 1000000;
  const operatingCostsFormated = `${operatingCosts.toString()} NYM`;

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
        label="Operating cost."
        value={operatingCostsFormated}
      />
    </ExplorerCard>
  );
};
