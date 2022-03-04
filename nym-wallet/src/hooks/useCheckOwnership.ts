import { useCallback, useContext, useEffect, useState } from 'react'
import { ClientContext } from '../context/main'
import { checkGatewayOwnership, checkMixnodeOwnership, getVestingPledgeInfo } from '../requests'
import { EnumNodeType, TNodeOwnership, PledgeData } from '../types'

const initial = {
  hasOwnership: false,
  nodeType: undefined,
  vestingPledge: undefined,
}

export const useCheckOwnership = () => {
  const { clientDetails } = useContext(ClientContext)

  const [ownership, setOwnership] = useState<TNodeOwnership>(initial)
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string>()

  const checkOwnership = useCallback(async () => {
    const status = {} as TNodeOwnership

    try {
      const [ownsMixnode, ownsGateway] = await Promise.all([checkMixnodeOwnership(), checkGatewayOwnership()])
      if (ownsMixnode) {
        status.hasOwnership = true
        status.nodeType = EnumNodeType.mixnode
        status.vestingPledge = await getVestingPledgeInfo({
          address: clientDetails?.client_address!,
          type: EnumNodeType.mixnode,
        })
      }

      if (ownsGateway) {
        status.hasOwnership = true
        status.nodeType = EnumNodeType.gateway
        status.vestingPledge = await getVestingPledgeInfo({
          address: clientDetails?.client_address!,
          type: EnumNodeType.gateway,
        })
      }

      setOwnership(status)
    } catch (e) {
      setError(e as string)
      setOwnership(initial)
    } finally {
      setIsLoading(false)
    }
  }, [clientDetails])

  useEffect(() => {
    checkOwnership()
  }, [clientDetails])

  return { isLoading, error, ownership, checkOwnership }
}
