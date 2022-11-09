import * as React from 'react';
import {
  ApiState,
  DelegationsResponse,
  UniqDelegationsResponse,
  MixNodeDescriptionResponse,
  MixNodeEconomicDynamicsStatsResponse,
  MixNodeResponseItem,
  StatsResponse,
  StatusResponse,
  UptimeStoryResponse,
} from '../typeDefs/explorer-api';
import { Api } from '../api';
import { useApiState } from './hooks';
import { mixNodeResponseItemToMixnodeRowType, MixnodeRowType } from '../components/MixNodes';

/**
 * This context provides the state for a single mixnode by identity key.
 */

interface MixnodeState {
  delegations?: ApiState<DelegationsResponse>;
  uniqDelegations?: ApiState<UniqDelegationsResponse>;
  description?: ApiState<MixNodeDescriptionResponse>;
  economicDynamicsStats?: ApiState<MixNodeEconomicDynamicsStatsResponse>;
  mixNode?: ApiState<MixNodeResponseItem | undefined>;
  mixNodeRow?: MixnodeRowType;
  stats?: ApiState<StatsResponse>;
  status?: ApiState<StatusResponse>;
  uptimeStory?: ApiState<UptimeStoryResponse>;
}

export const MixnodeContext = React.createContext<MixnodeState>({});

export const useMixnodeContext = (): React.ContextType<typeof MixnodeContext> =>
  React.useContext<MixnodeState>(MixnodeContext);

interface MixnodeContextProviderProps {
  mixNodeIdentityKey: string;
}

/**
 * Provides a state context for a mixnode by identity
 * @param mixNodeIdentityKey   The identity key of the mixnode
 */
export const MixnodeContextProvider: React.FC<MixnodeContextProviderProps> = ({ mixNodeIdentityKey, children }) => {
  const [mixNode, fetchMixnodeById, clearMixnodeById] = useApiState<MixNodeResponseItem | undefined>(
    mixNodeIdentityKey,
    Api.fetchMixnodeByID,
    'Failed to fetch mixnode by id',
  );

  const [mixNodeRow, setMixnodeRow] = React.useState<MixnodeRowType | undefined>();

  const [delegations, fetchDelegations, clearDelegations] = useApiState<DelegationsResponse>(
    mixNodeIdentityKey,
    Api.fetchDelegationsById,
    'Failed to fetch delegations for mixnode',
  );

  const [uniqDelegations, fetchUniqDelegations, clearUniqDelegations] = useApiState<UniqDelegationsResponse>(
    mixNodeIdentityKey,
    Api.fetchUniqDelegationsById,
    'Failed to fetch delegations for mixnode',
  );

  const [status, fetchStatus, clearStatus] = useApiState<StatusResponse>(
    mixNodeIdentityKey,
    Api.fetchStatusById,
    'Failed to fetch mixnode status',
  );

  const [stats, fetchStats, clearStats] = useApiState<StatsResponse>(
    mixNodeIdentityKey,
    Api.fetchStatsById,
    'Failed to fetch mixnode stats',
  );

  const [description, fetchDescription, clearDescription] = useApiState<MixNodeDescriptionResponse>(
    mixNodeIdentityKey,
    Api.fetchMixnodeDescriptionById,
    'Failed to fetch mixnode description',
  );

  const [economicDynamicsStats, fetchEconomicDynamicsStats, clearEconomicDynamicsStats] =
    useApiState<MixNodeEconomicDynamicsStatsResponse>(
      mixNodeIdentityKey,
      Api.fetchMixnodeEconomicDynamicsStatsById,
      'Failed to fetch mixnode dynamics stats by id',
    );

  const [uptimeStory, fetchUptimeHistory, clearUptimeHistory] = useApiState<UptimeStoryResponse>(
    mixNodeIdentityKey,
    Api.fetchUptimeStoryById,
    'Failed to fetch mixnode uptime history',
  );

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearMixnodeById();
    clearDelegations();
    clearUniqDelegations();
    clearStatus();
    clearStats();
    clearDescription();
    clearEconomicDynamicsStats();
    clearUptimeHistory();

    // fetch the mixnode, then get all the other stuff
    fetchMixnodeById().then((value) => {
      if (!value.data || value.error) {
        setMixnodeRow(undefined);
        return;
      }
      setMixnodeRow(mixNodeResponseItemToMixnodeRowType(value.data));
      Promise.all([
        fetchDelegations(),
        fetchUniqDelegations(),
        fetchStatus(),
        fetchStats(),
        fetchDescription(),
        fetchEconomicDynamicsStats(),
        fetchUptimeHistory(),
      ]);
    });
  }, [mixNodeIdentityKey]);

  const state = React.useMemo<MixnodeState>(
    () => ({
      delegations,
      uniqDelegations,
      mixNode,
      mixNodeRow,
      description,
      economicDynamicsStats,
      stats,
      status,
      uptimeStory,
    }),
    [
      {
        delegations,
        uniqDelegations,
        mixNode,
        mixNodeRow,
        description,
        economicDynamicsStats,
        stats,
        status,
        uptimeStory,
      },
    ],
  );

  return <MixnodeContext.Provider value={state}>{children}</MixnodeContext.Provider>;
};
