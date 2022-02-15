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

  const getStatus = async (mixnodeKey: string) => {
    const status = await getMixnodeStatus(mixnodeKey)
    setStatus(status.status)
  }

  const getStakeSaturation = async (mixnodeKey: string) => {
    const saturation = await getMixnodeStakeSaturation(mixnodeKey)

    if (saturation) {
      setSaturation(Math.round(saturation.saturation * 100))
    }
  }

  const getRewardEstimation = async (mixnodeKey: string) => {
    const rewardEstimation = await getMixnodeRewardEstimation(mixnodeKey)
    if (rewardEstimation) {
      const toMajor = await minorToMajor(rewardEstimation.estimated_total_node_reward.toString())
      setRewardEstimation(parseInt(toMajor.amount))
    }
  }

  const getMixnodeInclusionProbability = async (mixnodeKey: string) => {
    const probability = await getInclusionProbability(mixnodeKey)
    if (probability) {
      const in_active = Math.round(probability.in_active * 100)
      const in_reserve = Math.round(probability.in_reserve * 100)
      setInclusionProbability({ in_active, in_reserve })
    }
  }

  const reset = () => {
    setStatus('not_found')
    setSaturation(0)
    setRewardEstimation(0)
    setInclusionProbability({ in_active: 0, in_reserve: 0 })
  }

  useEffect(() => {
    if (shouldUpdate && mixnodeDetails?.mix_node.identity_key) {
      getStatus(mixnodeDetails?.mix_node.identity_key)
      getStakeSaturation(mixnodeDetails?.mix_node.identity_key)
      getRewardEstimation(mixnodeDetails?.mix_node.identity_key)
      getMixnodeInclusionProbability(mixnodeDetails?.mix_node.identity_key)
    } else {
      reset()
    }
  }, [shouldUpdate])

  return {
    status,
    saturation,
    rewardEstimation,
    inclusionProbability,
  }
}
