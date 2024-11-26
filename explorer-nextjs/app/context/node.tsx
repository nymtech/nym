'use client'

import * as React from 'react'
import {
  ApiState,
  NymNodeReportResponse,
  UptimeStoryResponse,
} from '@/app/typeDefs/explorer-api'
import { Api } from '@/app/api'
import { useApiState } from './hooks'

/**
 * This context provides the state for a single gateway by identity key.
 */

interface NymNodeState {
  uptimeReport?: ApiState<NymNodeReportResponse>
  uptimeHistory?: ApiState<UptimeStoryResponse>
}

export const NymNodeContext = React.createContext<NymNodeState>({})

export const useNymNodeContext = (): React.ContextType<typeof NymNodeContext> =>
  React.useContext<NymNodeState>(NymNodeContext)

/**
 * Provides a state context for a gateway by identity
 * @param gatewayIdentityKey   The identity key of the gateway
 */
export const NymNodeContextProvider = ({
  nymNodeId,
  children,
}: {
  nymNodeId: string
  children: JSX.Element
}) => {
  const [uptimeReport, fetchUptimeReportById, clearUptimeReportById] =
    useApiState<any>(
      nymNodeId,
      Api.fetchNymNodePerformanceById,
      'Failed to fetch gateway uptime report by id'
    )

  const [uptimeHistory, fetchUptimeHistory, clearUptimeHistory] =
    useApiState<UptimeStoryResponse>(
      nymNodeId,
      async (arg) => {
        const res = await Api.fetchNymNodeUptimeHistoryById(arg);
        const uptimeHistory: UptimeStoryResponse = {
          history: res.history.data,
          identity: '',
          owner: '',
        }
        return uptimeHistory;
      },
      'Failed to fetch gateway uptime history'
    )

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearUptimeReportById()
    clearUptimeHistory()
    Promise.all([fetchUptimeReportById(), fetchUptimeHistory()])
  }, [nymNodeId])

  const state = React.useMemo<NymNodeState>(
    () => ({
      uptimeReport,
      uptimeHistory,
    }),
    [uptimeReport, uptimeHistory]
  )

  return (
    <NymNodeContext.Provider value={state}>{children}</NymNodeContext.Provider>
  )
}
