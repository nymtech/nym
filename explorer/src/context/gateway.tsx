import * as React from 'react';
import { ApiState, GatewayReportResponse, UptimeStoryResponse } from '../typeDefs/explorer-api';
import { Api } from '../api';

/**
 * This context provides the state for a single gateway by identity key.
 */

interface GatewayState {
  uptimeReport?: ApiState<GatewayReportResponse>;
  //   uptimeStory?: ApiState<UptimeStoryResponse>;
}

export const GatewayContext = React.createContext<GatewayState>({});

export const useGatewayContext = (): React.ContextType<typeof GatewayContext> =>
  React.useContext<GatewayState>(GatewayContext);

interface GatewayContextProviderProps {
  gatewayIdentityKey: string;
}

/**
 * Provides a state context for a gateway by identity
 * @param gatewayIdentityKey   The identity key of the gateway
 */
export const GatewayContextProvider: React.FC<GatewayContextProviderProps> = ({ gatewayIdentityKey, children }) => {
  const [uptimeReport, fetchUptimeReportById, clearUptimeReportById] = useApiState<GatewayReportResponse>(
    gatewayIdentityKey,
    Api.fetchGatewayReportById,
    'Failed to fetch gateway uptime report by id',
  );

  //   const [uptimeStory, fetchUptimeHistory, clearUptimeHistory] = useApiState<UptimeStoryResponse>(
  //     gatewayIdentityKey,
  //     Api.fetchUptimeStoryById,
  //     'Failed to fetch gateway uptime history',
  //   );

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearUptimeReportById();
    Promise.all([fetchUptimeReportById()]);
  }, [gatewayIdentityKey]);

  const state = React.useMemo<GatewayState>(
    () => ({
      uptimeReport,
      //   uptimeStory,
    }),
    [
      {
        uptimeReport,
        // uptimeStory,
      },
    ],
  );

  return <GatewayContext.Provider value={state}>{children}</GatewayContext.Provider>;
};

/**
 * Custom hook to get data from the API by passing an id to a delegate method that fetches the data asynchronously
 * @param id                The id to fetch
 * @param fn                Delegate the fetching to this method (must take `(id: string)` as a parameter)
 * @param errorMessage      A static error message, to use when no dynamic error message is returned
 */
function useApiState<T>(
  id: string,
  fn: (id: string) => Promise<T>,
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
