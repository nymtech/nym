import { currencyToString } from '../../utils/currency';
import { useMixnodeContext } from '../../context/mixnode';

export const delegatorsInfoRows: any = () => {

    const { economicDynamicsStats } = useMixnodeContext();

    const estimatedDelegatorsReward = economicDynamicsStats?.data?.estimated_delegators_reward || 0;
    const estimatedNodeRewards = economicDynamicsStats?.data?.estimated_total_node_reward || 0;
    const activeSetProbability = economicDynamicsStats?.data?.active_set_inclusion_probability || 0;
    const stakeSaturation = economicDynamicsStats?.data?.stake_saturation || 0;
    const profitMargin = (estimatedDelegatorsReward / estimatedNodeRewards) * 100 || 0;


    return ({
    id: 1,
    estimated_reward: {
        value: currencyToString(estimatedDelegatorsReward.toString()),
    },
    active_set_probability: {
        value: `${activeSetProbability} %`,
        visualProgressValue: activeSetProbability,
    },
    stake_saturation: {
        value: `${stakeSaturation} %`,
        visualProgressValue: stakeSaturation,
    },
    profit_margin: {
        value: `${profitMargin} %`,
    },
})}