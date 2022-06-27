import { PaletteMode } from '@mui/material';
import * as React from 'react';
import {
  InclusionProbability,
  MixNodeBondWithDetails,
  RewardEstimation,
  RewardEstimationParams,
  RewardParams,
} from './types';

// const API_BASE = 'https://qa-validator-api.nymtech.net/api';
const API_BASE = 'https://validator-apy.dev.nymte.ch/api';

interface State {
  mode: PaletteMode;
  toggleMode: () => void;
  loading: boolean;
  mixnodes: MixNodeBondWithDetails[] | undefined;
  rewardParams: RewardParams | undefined;
}

const AppContext = React.createContext<State | undefined>(undefined);

export const Api = {
  computeRewardEstimation: async (identityKey: string, params: RewardEstimationParams): Promise<RewardEstimation> => {
    const response = await fetch(`${API_BASE}/v1/status/mixnode/${identityKey}/compute-reward-estimation`, {
      method: 'POST',
      body: JSON.stringify(params),
    });
    return response.json();
  },
  getMixnodesDetailed: async (): Promise<MixNodeBondWithDetails[]> => {
    const response = await fetch(`${API_BASE}/v1/mixnodes/detailed`);
    const items = (await response.json()) as MixNodeBondWithDetails[];
    const page = items
      .sort((a, b) => {
        const amountA = Number.parseFloat(a.mixnode_bond.total_delegation.amount);
        const amountB = Number.parseFloat(b.mixnode_bond.total_delegation.amount);
        return amountB - amountA;
      })
      .slice(0, 100);
    await Promise.all(
      page.map(async (item) => {
        const status = await Api.getMixnodeStatus(item.mixnode_bond.mix_node.identity_key);
        const probability = await Api.getMixnodeInclusionProbability(item.mixnode_bond.mix_node.identity_key);
        // eslint-disable-next-line no-param-reassign
        item.status = status;
        // eslint-disable-next-line no-param-reassign
        item.inclusion_probability = probability;
      }),
    );
    return page;
  },
  getRewardParams: async (): Promise<RewardParams> => {
    const response = await fetch(`${API_BASE}/v1/epoch/reward_params`);
    const params = (await response.json()) as RewardParams;
    return params;
  },
  getMixnodeStatus: async (identityKey: string): Promise<string> => {
    const response = await fetch(`${API_BASE}/v1/status/mixnode/${identityKey}/status`);
    return (await response.json()).status;
  },
  getMixnodeInclusionProbability: async (identityKey: string): Promise<InclusionProbability> => {
    const response = await fetch(`${API_BASE}/v1/status/mixnode/${identityKey}/inclusion-probability`);
    return (await response.json()) as InclusionProbability;
  },
};

export const useAppContext = (): State => {
  const context = React.useContext<State | undefined>(AppContext);

  if (!context) {
    throw new Error('Please include a `import { AppContextProvider } from "./context"` before using this hook');
  }

  return context;
};

export const AppContextProvider: React.FC = ({ children }) => {
  // light/dark mode
  const [mode, setMode] = React.useState<PaletteMode>('dark');

  const [loading, setLoading] = React.useState<boolean>(false);
  const [mixnodes, setMixnodes] = React.useState<MixNodeBondWithDetails[] | undefined>();
  const [rewardParams, setRewardParams] = React.useState<RewardParams | undefined>();

  const refresh = async () => {
    setMixnodes(await Api.getMixnodesDetailed());
    setRewardParams(await Api.getRewardParams());
  };

  React.useEffect(() => {
    setLoading(true);
    refresh().finally(() => setLoading(false));
  }, []);

  const value = React.useMemo<State>(
    () => ({
      mode,
      toggleMode: () => setMode((prevMode) => (prevMode !== 'light' ? 'light' : 'dark')),
      loading,
      mixnodes,
      rewardParams,
    }),
    [mode, mixnodes, loading, rewardParams],
  );

  return <AppContext.Provider value={value}>{children}</AppContext.Provider>;
};
