/* eslint-disable no-nested-ternary */
import React, { useContext, useState } from 'react';
import { useForm } from 'react-hook-form';
import { Box, Button, Chip, CircularProgress, Grid, LinearProgress, Stack, TextField, Typography } from '@mui/material';
import { PercentOutlined } from '@mui/icons-material';
import { yupResolver } from '@hookform/resolvers/yup';
import { InclusionProbabilityResponse, SelectionChance } from '@nymproject/types';
import { validationSchema } from './validationSchema';
import { InfoTooltip } from '../../components';
import { updateMixnodeCostParams } from '../../requests';
import { AppContext } from '../../context';
import { Console } from '../../utils/console';
import { attachDefaultOperatingCost, toPercentFloatString } from '../../utils';

const DataField = ({ title, info, Indicator }: { title: string; info: string; Indicator: React.ReactElement }) => (
  <Grid container justifyContent="space-between">
    <Grid item xs={12} md={6}>
      <Box display="flex" alignItems="center">
        <InfoTooltip title={info} tooltipPlacement="right" />
        <Typography sx={{ ml: 1 }}>{title}</Typography>
      </Box>
    </Grid>

    <Grid item xs={12} md={6}>
      <Box display="flex" justifyContent="flex-end">
        {Indicator}
      </Box>
    </Grid>
  </Grid>
);

const colorMap: { [key in SelectionChance]: string } = {
  Low: 'error.main',
  Good: 'warning.main',
  High: 'success.main',
};

const textMap: { [key in SelectionChance]: string } = {
  Low: 'Low',
  Good: 'Good',
  High: 'High',
};

const InclusionProbability = ({ probability }: { probability: SelectionChance }) => (
  <Typography sx={{ color: colorMap[probability] }}>{textMap[probability]}</Typography>
);

const PercentIndicator = ({ value, warning }: { value: number; warning?: boolean }) => (
  <Grid container alignItems="center">
    <Grid item xs={2}>
      <Typography component="span" sx={{ color: warning ? 'error.main' : 'nym.fee', fontWeight: 600 }}>
        {value}%
      </Typography>
    </Grid>
    <Grid item xs={10}>
      <LinearProgress
        color="inherit"
        sx={{ color: warning ? 'error.main' : 'nym.fee' }}
        variant="determinate"
        value={value < 100 ? value : 100}
      />
    </Grid>
  </Grid>
);

export const SystemVariables = ({
  saturation,
  inclusionProbability,
}: {
  saturation: number;
  rewardEstimation: number;
  inclusionProbability: InclusionProbabilityResponse;
}) => {
  const [nodeUpdateResponse, setNodeUpdateResponse] = useState<'success' | 'failed'>();
  const { mixnodeDetails } = useContext(AppContext);

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm({
    resolver: yupResolver(validationSchema),
    defaultValues: { profitMarginPercent: mixnodeDetails?.rewarding_details.cost_params.profit_margin_percent },
  });

  const onSubmit = async (
    profitMarginPercent: string | undefined,
    cb: (profitMarginPercent: string) => Promise<any>,
  ) => {
    if (profitMarginPercent) {
      try {
        await cb(profitMarginPercent);
        setNodeUpdateResponse('success');
      } catch (e) {
        setNodeUpdateResponse('failed');
        Console.log(e as string);
      }
    }
  };

  const updateMixnodeProfitMargin = async (profitMarginPercent: string) => {
    // TODO: this will have to be updated with allowing users to provide their operating cost in the form
    const defaultCostParams = await attachDefaultOperatingCost(toPercentFloatString(profitMarginPercent));
    await updateMixnodeCostParams(defaultCostParams);
  };

  if (!mixnodeDetails) return null;

  return (
    <>
      <Box sx={{ p: 3 }}>
        <Stack spacing={5}>
          <TextField
            {...register('profitMarginPercent', { valueAsNumber: true })}
            label="Profit margin"
            helperText={
              errors.profitMarginPercent
                ? errors.profitMarginPercent.message
                : "The percentage of your delegators' rewards that you as the node operator will take"
            }
            InputProps={{
              endAdornment: <PercentOutlined fontSize="small" sx={{ color: (theme) => theme.palette.text.disabled }} />,
            }}
            error={!!errors.profitMarginPercent}
            disabled={isSubmitting}
            InputLabelProps={{ shrink: true }}
          />
          <DataField
            title="Estimated reward"
            info="Estimated reward per epoch for this profit margin if your node is selected in the active set."
            Indicator={<Chip label="Coming soon" />}
          />

          <DataField
            title="Estimated chance of being in the active set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<InclusionProbability probability={inclusionProbability.in_active} />}
          />
          <DataField
            title="Estimated chance of being in the standby set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<InclusionProbability probability={inclusionProbability.in_reserve} />}
          />

          <DataField
            title="Node stake saturation"
            info="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 750k NYM, computed as S/K where S is the total amount of tokens available to stakeholders and K is the number of nodes in the reward set."
            Indicator={<PercentIndicator value={saturation} warning={saturation >= 100} />}
          />
        </Stack>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          p: 3,
          pt: 0,
        }}
      >
        {nodeUpdateResponse === 'success' && (
          <Typography sx={{ color: 'success.main', fontWeight: 600 }}>Node successfully updated</Typography>
        )}
        {nodeUpdateResponse === 'failed' && (
          <Typography sx={{ color: 'error.main', fontWeight: 600 }}>Node update failed</Typography>
        )}
        <Button
          variant="contained"
          color="primary"
          onClick={handleSubmit((data) => onSubmit(data.profitMarginPercent, updateMixnodeProfitMargin))}
          disableElevation
          endIcon={isSubmitting && <CircularProgress size={20} />}
          disabled={Object.keys(errors).length > 0 || isSubmitting}
          size="large"
        >
          Update Profit Margin
        </Button>
      </Box>
    </>
  );
};
