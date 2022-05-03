import { useContext, useEffect, useState } from 'react';
import { ClientContext } from '../../context/main';
import { getMixnodeStakeSaturation, getMixnodeStatus, getInclusionProbability } from '../../requests';
import { MixnodeStatus, InclusionProbabilityResponse } from '../../types';

export const useSettingsState = (shouldUpdate: boolean) => {
  const [status, setStatus] = useState<MixnodeStatus>('not_found');
  const [saturation, setSaturation] = useState<number>(0);
  const [rewardEstimation, setRewardEstimation] = useState<number>(0);
  const [inclusionProbability, setInclusionProbability] = useState<InclusionProbabilityResponse>({
    in_active: 'Low',
    in_reserve: 'Low',
  });

  const { mixnodeDetails } = useContext(ClientContext);

  const getStatus = async (mixnodeKey: string) => {
    const newStatus = await getMixnodeStatus(mixnodeKey);
    setStatus(newStatus.status);
  };

  const getStakeSaturation = async (mixnodeKey: string) => {
    const newSaturation = await getMixnodeStakeSaturation(mixnodeKey);

    if (newSaturation) {
      setSaturation(Math.round(newSaturation.saturation * 100));
    }
  };

  const getMixnodeInclusionProbability = async (mixnodeKey: string) => {
    const probability = await getInclusionProbability(mixnodeKey);
    if (probability) {
      // eslint-disable-next-line @typescript-eslint/naming-convention

      setInclusionProbability({ in_active: probability.in_active, in_reserve: probability.in_reserve });
    }
  };

  const reset = () => {
    setStatus('not_found');
    setSaturation(0);
    setRewardEstimation(0);
    setInclusionProbability({ in_active: 'Low', in_reserve: 'Low' });
  };

  useEffect(() => {
    if (shouldUpdate && mixnodeDetails?.mix_node.identity_key) {
      (async () => {
        await getStatus(mixnodeDetails?.mix_node.identity_key);
        await getStakeSaturation(mixnodeDetails?.mix_node.identity_key);
        await getMixnodeInclusionProbability(mixnodeDetails?.mix_node.identity_key);
      })();
    } else {
      reset();
    }
  }, [shouldUpdate]);

  return {
    status,
    saturation,
    rewardEstimation,
    inclusionProbability,
  };
};
