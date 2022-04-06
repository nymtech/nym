const estimatedReward = 12345;
const activeSetProbability = 45;
const stakeSaturation = 10;
const profitMargin = 12;

export const delegatorsInfoRows: any = {
    id: 1,
    estimated_reward: {
        value: `${estimatedReward} NYM` || 0,
    },
    active_set_probability: {
        value: `${activeSetProbability} %` || 0,
        visualProgressValue: activeSetProbability,
    },
    stake_saturation: {
        value: `${stakeSaturation} %` || 0,
        visualProgressValue: stakeSaturation || 0,
    },
    profit_margin: {
        value: `${profitMargin} %` || 0,
    },
}