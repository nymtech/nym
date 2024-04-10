'use client'

import * as React from 'react'
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
} from '../typeDefs/explorer-api'
import { Api } from '../api'
import { useApiState } from './hooks'
import {
  mixNodeResponseItemToMixnodeRowType,
  MixnodeRowType,
} from '../components/MixNodes'

/**
 * This context provides the state for a single mixnode by identity key.
 */

interface MixnodeState {
  delegations?: ApiState<DelegationsResponse>
  uniqDelegations?: ApiState<UniqDelegationsResponse>
  description?: ApiState<MixNodeDescriptionResponse>
  economicDynamicsStats?: ApiState<MixNodeEconomicDynamicsStatsResponse>
  mixNode?: ApiState<MixNodeResponseItem | undefined>
  mixNodeRow?: MixnodeRowType
  stats?: ApiState<StatsResponse>
  status?: ApiState<StatusResponse>
  uptimeStory?: ApiState<UptimeStoryResponse>
}

export const MixnodeContext = React.createContext<MixnodeState>({})

export const useMixnodeContext = (): React.ContextType<typeof MixnodeContext> =>
  React.useContext<MixnodeState>(MixnodeContext)

interface MixnodeContextProviderProps {
  mixId: string
  children: React.ReactNode
}

/**
 * Provides a state context for a mixnode by identity
 * @param mixId   The mixID of the mixnode
 */
export const MixnodeContextProvider: FCWithChildren<
  MixnodeContextProviderProps
> = ({ mixId, children }) => {
  const [mixNode, fetchMixnodeById, clearMixnodeById] = useApiState<
    MixNodeResponseItem | undefined
  >(mixId, Api.fetchMixnodeByID, 'Failed to fetch mixnode by id')

  const [mixNodeRow, setMixnodeRow] = React.useState<
    MixnodeRowType | undefined
  >()

  const [delegations, fetchDelegations, clearDelegations] =
    useApiState<DelegationsResponse>(
      mixId,
      Api.fetchDelegationsById,
      'Failed to fetch delegations for mixnode'
    )

  const [uniqDelegations, fetchUniqDelegations, clearUniqDelegations] =
    useApiState<UniqDelegationsResponse>(
      mixId,
      Api.fetchUniqDelegationsById,
      'Failed to fetch delegations for mixnode'
    )

  const [status, fetchStatus, clearStatus] = useApiState<StatusResponse>(
    mixId,
    Api.fetchStatusById,
    'Failed to fetch mixnode status'
  )

  const [stats, fetchStats, clearStats] = useApiState<StatsResponse>(
    mixId,
    Api.fetchStatsById,
    'Failed to fetch mixnode stats'
  )

  const [description, fetchDescription, clearDescription] =
    useApiState<MixNodeDescriptionResponse>(
      mixId,
      Api.fetchMixnodeDescriptionById,
      'Failed to fetch mixnode description'
    )

  const [
    economicDynamicsStats,
    fetchEconomicDynamicsStats,
    clearEconomicDynamicsStats,
  ] = useApiState<MixNodeEconomicDynamicsStatsResponse>(
    mixId,
    Api.fetchMixnodeEconomicDynamicsStatsById,
    'Failed to fetch mixnode dynamics stats by id'
  )

  const [uptimeStory, fetchUptimeHistory, clearUptimeHistory] =
    useApiState<UptimeStoryResponse>(
      mixId,
      Api.fetchUptimeStoryById,
      'Failed to fetch mixnode uptime history'
    )

  React.useEffect(() => {
    // when the identity key changes, remove all previous data
    clearMixnodeById()
    clearDelegations()
    clearUniqDelegations()
    clearStatus()
    clearStats()
    clearDescription()
    clearEconomicDynamicsStats()
    clearUptimeHistory()

    // fetch the mixnode, then get all the other stuff
    fetchMixnodeById().then((value) => {
      if (!value.data || value.error) {
        setMixnodeRow(undefined)
        return
      }
      setMixnodeRow(mixNodeResponseItemToMixnodeRowType(value.data))
      Promise.all([
        fetchDelegations(),
        fetchUniqDelegations(),
        fetchStatus(),
        fetchStats(),
        fetchDescription(),
        fetchEconomicDynamicsStats(),
        fetchUptimeHistory(),
      ])
    })
  }, [mixId])

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
    ]
  )

  return (
    <MixnodeContext.Provider value={state}>{children}</MixnodeContext.Provider>
  )
}
