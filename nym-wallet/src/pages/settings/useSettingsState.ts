import { useContext, useEffect, useState } from 'react'
import { ClientContext } from '../../context/main'
import { getMixnodeBondDetails, getMixnodeStakeSaturation, getMixnodeStatus } from '../../requests'
import { TMixnodeBondDetails, StakeSaturationResponse, MixnodeStatusResponse } from '../../types'

export const useSettingsState = (showSettings: boolean) => {
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()
  const [status, setStatus] = useState<MixnodeStatusResponse>()
  const [saturation, setSaturation] = useState<StakeSaturationResponse>()

  const { clientDetails } = useContext(ClientContext)

  const getBondDetails = async () => {
    const details = await getMixnodeBondDetails()
    setMixnodeDetails(details)
  }

  const getStatus = async () => {
    if (clientDetails?.client_address) {
      const status = await getMixnodeStatus(clientDetails?.contract_address)
      setStatus(status)
    }
  }

  const getStakeSaturation = async () => {
    if (clientDetails?.client_address) {
      const saturation = await getMixnodeStakeSaturation(clientDetails?.contract_address)
      setSaturation(saturation)
    }
  }

  useEffect(() => {
    if (showSettings) {
      getBondDetails()
      getStatus()
      getStakeSaturation()
    }
  }, [showSettings])

  return {
    status,
    saturation,
    mixnodeDetails,
  }
}
