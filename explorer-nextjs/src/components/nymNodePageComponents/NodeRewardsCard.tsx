import type { IBondInfo, INodeDescription } from "@/app/api";
import { Stack } from "@mui/material";
import { format } from "date-fns";
import ExplorerCard from "../cards/ExplorerCard";
import ExplorerListItem from "../list/ListItem";

interface INodeRewardsCardProps {
  bondInfo: IBondInfo;
}

export const NodeRewardsCard = (props: INodeRewardsCardProps) => {
  const { bondInfo } = props;

  const totalRewards =
    Number(bondInfo.rewarding_details.total_unit_reward) / 1000000;
  const totalRewardsFormated = `${totalRewards} NYM`;

  const operatorRewards = Number(bondInfo.rewarding_details.operator) / 1000000;
  const operatorRewardsFormated = `${operatorRewards} NYM`;

  const stakerRewards = Number(bondInfo.rewarding_details.delegates) / 1000000;
  const stakerRewardsFormated = `${stakerRewards} NYM`;

  const profitMarginPercent =
    Number(bondInfo.rewarding_details.cost_params.profit_margin_percent) * 100;

  const profitMarginPercentFormated = `${profitMarginPercent}%`;

  const operatingCosts =
    Number(
      bondInfo.rewarding_details.cost_params.interval_operating_cost.amount,
    ) / 1000000;
  const operatingCostsFormated = `${operatingCosts.toString()} NYM`;

  return (
    <ExplorerCard label="Node rewards(last epoch/hour)" sx={{ height: "100%" }}>
      <ExplorerListItem
        row
        divider
        label="Total rew."
        value={totalRewardsFormated}
      />
      <ExplorerListItem
        row
        divider
        label="Operator rew."
        value={operatorRewardsFormated}
      />
      <ExplorerListItem
        row
        divider
        label="Staker rew."
        value={stakerRewardsFormated}
      />
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
