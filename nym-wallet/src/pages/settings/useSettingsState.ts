import { useContext, useEffect, useState } from 'react'
import { ClientContext } from '../../context/main'
import {
  getMixnodeRewardEstimation,
  getMixnodeStakeSaturation,
  getMixnodeStatus,
  minorToMajor,
  getInclusionProbability,
} from '../../requests'
import { MixnodeStatus, InclusionProbabilityResponse } from '../../types'

export const useSettingsState = (shouldUpdate: boolean) => {
  const [status, setStatus] = useState<MixnodeStatus>('not_found')
  const [saturation, setSaturation] = useState<number>(0)
  const [rewardEstimation, setRewardEstimation] = useState<number>(0)
  const [inclusionProbability, setInclusionProbability] = useState<InclusionProbabilityResponse>({
    in_active: 0,
    in_reserve: 0,
  })

  const { mixnodeDetails } = useContext(ClientContext)

  const getStatus = async () => {
    if (mixnodeDetails?.mix_node.identity_key) {
      const status = await getMixnodeStatus(mixnodeDetails?.mix_node.identity_key)
      setStatus(status.status)
    }
  }

  const getStakeSaturation = async () => {
    if (mixnodeDetails?.mix_node.identity_key) {
      const saturation = await getMixnodeStakeSaturation(mixnodeDetails?.mix_node.identity_key)

      if (saturation) {
        setSaturation(Math.round(saturation.saturation * 100))
      }
    }
  }

  const getRewardEstimation = async () => {
    if (mixnodeDetails?.mix_node.identity_key) {
      const rewardEstimation = await getMixnodeRewardEstimation(mixnodeDetails?.mix_node.identity_key)
      if (rewardEstimation) {
        const toMajor = await minorToMajor(rewardEstimation.estimated_total_node_reward.toString())
        setRewardEstimation(parseInt(toMajor.amount))
      }
    }
  }

  const getMixnodeInclusionProbability = async () => {
    if (mixnodeDetails?.mix_node.identity_key) {
      const probability = await getInclusionProbability(mixnodeDetails?.mix_node.identity_key)
      if (probability) {
        const in_active = Math.round(probability.in_active * 100)
        const in_reserve = Math.round(probability.in_reserve * 100)
        setInclusionProbability({ in_active, in_reserve })
      }
    }
  }

  useEffect(() => {
    if (shouldUpdate) {
      getStatus()
      getStakeSaturation()
      getRewardEstimation()
      getMixnodeInclusionProbability()
    }
  }, [shouldUpdate])

  return {
    status,
    saturation,
    rewardEstimation,
    inclusionProbability,
  }
}
