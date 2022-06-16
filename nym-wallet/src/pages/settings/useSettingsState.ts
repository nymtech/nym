import { useState } from 'react';
import { useSnackbar } from 'notistack';
import { InclusionProbabilityResponse, MixnodeStatus } from '@nymproject/types';
import { getInclusionProbability, getMixnodeStakeSaturation, getMixnodeStatus } from '../../requests';

export const useSettingsState = () => {
  const [status, setStatus] = useState<MixnodeStatus>('not_found');
  const [saturation, setSaturation] = useState<number>(0);
  const [rewardEstimation, setRewardEstimation] = useState<number>(0);
  const [inclusionProbability, setInclusionProbability] = useState<InclusionProbabilityResponse>({
    in_active: 'Low',
    in_reserve: 'Low',
  });

  const { enqueueSnackbar } = useSnackbar();

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

  const updateAllMixnodeStats = async (mixnodeId: string) => {
    try {
      await getStatus(mixnodeId);
      await getStakeSaturation(mixnodeId);
      await getMixnodeInclusionProbability(mixnodeId);
    } catch (e) {
      enqueueSnackbar(e as string, { variant: 'error', preventDuplicate: true });
      reset();
    }
  };

  return {
    status,
    saturation,
    rewardEstimation,
    inclusionProbability,
    updateAllMixnodeStats,
  };
};
