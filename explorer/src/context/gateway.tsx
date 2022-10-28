import * as React from 'react';
import { ApiState, GatewayReportResponse, UptimeStoryResponse } from '../typeDefs/explorer-api';
import { Api } from '../api';
import { useApiState } from './hooks';

/**
 * This context provides the state for a single gateway by identity key.
 */

interface GatewayState {
  uptimeReport?: ApiState<GatewayReportResponse>;
  uptimeStory?: ApiState<UptimeStoryResponse>;
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

  const [uptimeStory, fetchUptimeHistory, clearUptimeHistory] = useApiState<UptimeStoryResponse>(
    gatewayIdentityKey,
    Api.fetchGatewayUptimeStoryById,
    'Failed to fetch gateway uptime history',
  );

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearUptimeReportById();
    clearUptimeHistory();
    Promise.all([fetchUptimeReportById(), fetchUptimeHistory()]);
  }, [gatewayIdentityKey]);

  const state = React.useMemo<GatewayState>(
    () => ({
      uptimeReport,
      uptimeStory,
    }),
    [
      {
        uptimeReport,
        uptimeStory,
      },
    ],
  );

  return <GatewayContext.Provider value={state}>{children}</GatewayContext.Provider>;
};
