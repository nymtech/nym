import { useState } from 'react';
import { useSnackbar } from 'notistack';
import { decimalToPercentage, InclusionProbabilityResponse, MixnodeStatus } from '@nymproject/types';
import { getInclusionProbability, getMixnodeStakeSaturation, getMixnodeStatus } from '../../requests';

export const useSettingsState = () => {
  const [status, setStatus] = useState<MixnodeStatus>('not_found');
  const [saturation, setSaturation] = useState<string>('-');
  const [rewardEstimation, setRewardEstimation] = useState<number>(0);
  const [inclusionProbability, setInclusionProbability] = useState<InclusionProbabilityResponse>({
    in_active: 'Low',
    in_reserve: 'Low',
  });

  const { enqueueSnackbar } = useSnackbar();

  const getStatus = async (mixId: number) => {
    const newStatus = await getMixnodeStatus(mixId);
    setStatus(newStatus.status);
  };

  const getStakeSaturation = async (mixId: number) => {
    const newSaturation = await getMixnodeStakeSaturation(mixId);

    if (newSaturation) {
      setSaturation(decimalToPercentage(newSaturation.uncapped_saturation));
    }
  };

  const getMixnodeInclusionProbability = async (mixId: number) => {
    const probability = await getInclusionProbability(mixId);
    if (probability) {
      // eslint-disable-next-line @typescript-eslint/naming-convention
      setInclusionProbability({ in_active: probability.in_active, in_reserve: probability.in_reserve });
    }
  };

  const reset = () => {
    setStatus('not_found');
    setSaturation('-');
    setRewardEstimation(0);
    setInclusionProbability({ in_active: 'Low', in_reserve: 'Low' });
  };

  const updateAllMixnodeStats = async (mixId: number) => {
    try {
      await getStatus(mixId);
      await getStakeSaturation(mixId);
      await getMixnodeInclusionProbability(mixId);
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
