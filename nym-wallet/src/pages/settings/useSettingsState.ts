import { useContext, useEffect, useState } from 'react'
import { ClientContext } from '../../context/main'
import {
  getMixnodeBondDetails,
  getMixnodeRewardEstimation,
  getMixnodeStakeSaturation,
  getMixnodeStatus,
  minorToMajor,
} from '../../requests'
import { TMixnodeBondDetails, MixnodeStatus } from '../../types'

export const useSettingsState = (showSettings: boolean) => {
  const [mixnodeDetails, setMixnodeDetails] = useState<TMixnodeBondDetails | null>()
  const [status, setStatus] = useState<MixnodeStatus>('NotFound')
  const [saturation, setSaturation] = useState<number>(0)
  const [rewardEstimation, setRewardEstimation] = useState<number>(0)

  const { clientDetails } = useContext(ClientContext)

  const getBondDetails = async () => {
    const details = await getMixnodeBondDetails()
    setMixnodeDetails(details)
  }

  const getStatus = async () => {
    if (clientDetails?.client_address) {
      const status = await getMixnodeStatus(clientDetails?.contract_address)
      setStatus(status.status)
    }
  }

  const getStakeSaturation = async () => {
    if (clientDetails?.client_address) {
      const saturation = await getMixnodeStakeSaturation(clientDetails?.contract_address)
      if (saturation) {
        setSaturation(Math.round(saturation.saturation * 100))
      }
    }
  }

  const getRewardEstimation = async () => {
    if (clientDetails?.client_address) {
      const rewardEstimation = await getMixnodeRewardEstimation(clientDetails?.contract_address)
      if (rewardEstimation) {
        const toMajor = await minorToMajor(rewardEstimation.estimated_total_node_reward.toString())
        setRewardEstimation(parseInt(toMajor.amount))
      }
    }
  }

  useEffect(() => {
    if (showSettings) {
      getBondDetails()
      getStatus()
      getStakeSaturation()
      getRewardEstimation()
    }
  }, [showSettings])

  return {
    status,
    saturation,
    mixnodeDetails,
    rewardEstimation,
  }
}
