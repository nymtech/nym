import * as React from 'react';
import {
  ApiState,
  DelegationsResponse,
  MixNodeDescriptionResponse,
  MixNodeResponseItem,
  StatsResponse,
  StatusResponse,
  UptimeStoryResponse,
} from 'src/typeDefs/explorer-api';
import { Api } from '../api';
import {
  mixNodeResponseItemToMixnodeRowType,
  MixnodeRowType,
} from '../components/MixNodes';

/**
 * This context provides the state for a single mixnode by identity key.
 */

interface MixnodeState {
  delegations?: ApiState<DelegationsResponse>;
  description?: ApiState<MixNodeDescriptionResponse>;
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
export const MixnodeContextProvider: React.FC<MixnodeContextProviderProps> = ({
  mixNodeIdentityKey,
  children,
}) => {
  const [mixNode, fetchMixnodeById, clearMixnodeById] = useApiState<
    MixNodeResponseItem | undefined
  >(mixNodeIdentityKey, Api.fetchMixnodeByID, 'Failed to fetch mixnode by id');

  const [mixNodeRow, setMixnodeRow] = React.useState<
    MixnodeRowType | undefined
  >();

  const [delegations, fetchDelegations, clearDelegations] =
    useApiState<DelegationsResponse>(
      mixNodeIdentityKey,
      Api.fetchDelegationsById,
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

  const [description, fetchDescription, clearDescription] =
    useApiState<MixNodeDescriptionResponse>(
      mixNodeIdentityKey,
      Api.fetchMixnodeDescriptionById,
      'Failed to fetch mixnode description',
    );

  const [uptimeStory, fetchUptimeHistory, clearUptimeHistory] =
    useApiState<UptimeStoryResponse>(
      mixNodeIdentityKey,
      Api.fetchUptimeStoryById,
      'Failed to fetch mixnode uptime history',
    );

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearMixnodeById();
    clearDelegations();
    clearStatus();
    clearStats();
    clearDescription();
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
        fetchStatus(),
        fetchStats(),
        fetchDescription(),
        fetchUptimeHistory(),
      ]);
    });
  }, [mixNodeIdentityKey]);

  return (
    <MixnodeContext.Provider
      value={{
        delegations,
        mixNode,
        mixNodeRow,
        description,
        stats,
        status,
        uptimeStory,
      }}
    >
      {children}
    </MixnodeContext.Provider>
  );
};

/**
 * Custom hook to get data from the API by passing an id to a delegate method that fetches the data asynchronously
 * @param id                The id to fetch
 * @param fn                Delegate the fetching to this method (must take `(id: string)` as a parameter)
 * @param errorMessage      A static error message, to use when no dynamic error message is returned
 */
function useApiState<T>(
  id: string,
  fn: (argId: string) => Promise<T>,
  errorMessage: string,
): [ApiState<T> | undefined, () => Promise<ApiState<T>>, () => void] {
  // stores the state
  const [value, setValue] = React.useState<ApiState<T> | undefined>();

  // clear the value
  const clearValueFn = () => setValue(undefined);

  // this provides a method to trigger the delegate to fetch data
  const wrappedFetchFn = React.useCallback(async () => {
    try {
      // keep previous state and set to loading
      setValue((prevState) => ({ ...prevState, isLoading: true }));

      // delegate to user function to get data and set if successful
      const data = await fn(id);
      const newValue: ApiState<T> = {
        isLoading: false,
        data,
      };
      setValue(newValue);
      return newValue;
    } catch (error) {
      // return the caught error or create a new error with the static error message
      const newValue: ApiState<T> = {
        error: error instanceof Error ? error : new Error(errorMessage),
        isLoading: false,
      };
      setValue(newValue);
      return newValue;
    }
  }, [setValue, fn]);
  return [value || { isLoading: true }, wrappedFetchFn, clearValueFn];
}
