import { useCallback, useState } from 'react'
import { checkGatewayOwnership, checkMixnodeOwnership } from '../requests'
import { EnumNodeType, TNodeOwnership } from '../types'

export const useCheckOwnership = () => {
  const [ownership, setOwnership] = useState<TNodeOwnership>({
    hasOwnership: false,
    nodeType: undefined,
  })
  const [isLoading, setIsLoading] = useState(false)
  const [error, setError] = useState<string>()

  const checkOwnership = useCallback(async () => {
    const status = {} as TNodeOwnership

    setIsLoading(true)

    try {
      const ownsMixnode = await checkMixnodeOwnership()
      const ownsGateway = await checkGatewayOwnership()

      if (true) {
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
    }
  }, [])

  return { isLoading, error, ownership, checkOwnership }
}
