import { invoke } from "@tauri-apps/api";

export const claimDelegatorRewards = async () => {
    const res: string = await invoke('claim_delegator_rewards');
    console.log(res);
    return res;
};