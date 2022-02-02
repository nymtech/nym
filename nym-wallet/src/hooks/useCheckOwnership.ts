import { useCallback, useContext, useEffect, useState } from 'react'
import { ClientContext } from '../context/main'
import { checkGatewayOwnership, checkMixnodeOwnership } from '../requests'
import { EnumNodeType, TNodeOwnership } from '../types'

const initial = {
  hasOwnership: false,
  nodeType: undefined,
}

export const useCheckOwnership = () => {
  const { clientDetails } = useContext(ClientContext)
  const [ownership, setOwnership] = useState<TNodeOwnership>(initial)
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string>()

  const checkOwnership = useCallback(async () => {
    const status = {} as TNodeOwnership

    setIsLoading(true)

    try {
      const ownsMixnode = await checkMixnodeOwnership()
      const ownsGateway = await checkGatewayOwnership()

      if (ownsMixnode) {
        status.hasOwnership = true
        status.nodeType = EnumNodeType.mixnode
      }

      if (ownsGateway) {
        status.hasOwnership = true
        status.nodeType = EnumNodeType.gateway
      }

      setOwnership(status)
    } catch (e) {
      setError(e as string)
      setIsLoading(false)
      setOwnership(initial)
    }
  }, [])

  useEffect(() => {
    checkOwnership()
  }, [clientDetails])

  return { isLoading, error, ownership, checkOwnership }
}
