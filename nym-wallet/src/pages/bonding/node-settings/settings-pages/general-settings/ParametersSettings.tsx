import React, { useContext, useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Button, Divider, FormHelperText, Grid, InputAdornment, TextField, Typography } from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { CurrencyDenom, MixNodeCostParams } from '@nymproject/types';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { isMixnode } from 'src/types';
import {
  getPendingIntervalEvents,
  simulateUpdateMixnodeCostParams,
  simulateVestingUpdateMixnodeCostParams,
  updateMixnodeCostParams,
  vestingUpdateMixnodeCostParams,
} from 'src/requests';
import { TBondedMixnode } from 'src/context/bonding';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { bondedNodeParametersValidationSchema } from 'src/components/Bonding/forms/mixnodeValidationSchema';
import { Console } from 'src/utils/console';
import { getIntervalAsDate } from 'src/utils';
import { Alert } from 'src/components/Alert';
import { ChangeMixCostParams } from 'src/pages/bonding/types';
import { AppContext } from 'src/context';
import { useGetFee } from 'src/hooks/useGetFee';
import { ConfirmTx } from 'src/components/ConfirmTX';
import { LoadingModal } from 'src/components/Modals/LoadingModal';

export const ParametersSettings = ({ bondedNode }: { bondedNode: TBondedMixnode }): JSX.Element => {
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [intervalTime, setIntervalTime] = useState<string>();
  const [pendingUpdates, setPendingUpdates] = useState<MixNodeCostParams>();
  const { clientDetails } = useContext(AppContext);
  const theme = useTheme();

  const { fee, getFee, resetFeeState } = useGetFee();

  const defaultValues = {
    operatorCost: bondedNode.operatorCost,
    profitMargin: bondedNode.profitMargin,
  };

  const {
    register,
    handleSubmit,
    reset,
    setValue,
    formState: { errors, isSubmitting, isDirty, isValid },
  } = useForm({
    resolver: yupResolver(bondedNodeParametersValidationSchema),
    mode: 'onChange',
    defaultValues,
  });

  const getCurrentInterval = async () => {
    try {
      const { nextInterval } = await getIntervalAsDate();
      setIntervalTime(nextInterval);
    } catch {
      console.log('cant retrieve next interval');
    }
  };

  const getPendingEvents = async () => {
    const events = await getPendingIntervalEvents();
    const latestEvent = events
      .reverse()
      .find(
        (evt) => 'ChangeMixCostParams' in evt.event && evt.event.ChangeMixCostParams.mix_id === bondedNode.mixId,
      ) as unknown as
      | {
          id: number;
          event: {
            ChangeMixCostParams: ChangeMixCostParams;
          };
        }
      | undefined;

    if (latestEvent) {
      setPendingUpdates(latestEvent.event.ChangeMixCostParams.new_costs);
    }
  };

  useEffect(() => {
    getCurrentInterval();
    getPendingEvents();
  }, []);

  const onSubmit = async (data: { operatorCost: { amount: string; denom: CurrencyDenom }; profitMargin: string }) => {
    resetFeeState();
    if (data.operatorCost && data.profitMargin) {
      const mixNodeCostParams = {
        profit_margin_percent: (+data.profitMargin / 100).toString(),
        interval_operating_cost: {
          amount: data.operatorCost.amount,
          denom: data.operatorCost.denom,
        },
      };
      try {
        if (bondedNode.proxy) {
          await vestingUpdateMixnodeCostParams(mixNodeCostParams);
        } else {
          await updateMixnodeCostParams(mixNodeCostParams);
        }
        await getPendingEvents();
        reset();
        setOpenConfirmationModal(true);
      } catch (error) {
        Console.error(error);
      }
    }
  };

  return (
    <Grid
      container
      xs
      item
      sx={{
        '& .MuiGrid-item': {
          pl: 0,
        },
      }}
    >
      {fee && (
        <ConfirmTx
          open
          header="Update cost parameters"
          fee={fee}
          onConfirm={handleSubmit((d) => onSubmit(d))}
          onPrev={resetFeeState}
          onClose={resetFeeState}
        />
      )}
      {isSubmitting && <LoadingModal />}
      <Alert title={<Box component="span" sx={{ fontWeight: 600 }}>{`Next interval: ${intervalTime}`}</Box>} />
      <Grid container direction="column">
        <Grid item container alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item xl={6}>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Profit Margin
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Changes to PM will be applied in the next interval.
            </Typography>
          </Grid>
          {isMixnode(bondedNode) && (
            <Grid item xs={12} md={6}>
              <TextField
                {...register('profitMargin')}
                name="profitMargin"
                label="Profit margin"
                fullWidth
                error={!!errors.profitMargin}
                helperText={errors.profitMargin?.message}
                InputProps={{
                  endAdornment: (
                    <InputAdornment position="end">
                      <Box>%</Box>
                    </InputAdornment>
                  ),
                }}
                InputLabelProps={{ shrink: true }}
              />
              {pendingUpdates && (
                <FormHelperText>
                  Your last change to{' '}
                  <Typography variant="caption" fontWeight="bold">
                    {(+pendingUpdates.profit_margin_percent * 100).toFixed(2)}%{' '}
                  </Typography>
                  will be applied in the next interval
                </FormHelperText>
              )}
            </Grid>
          )}
        </Grid>
        <Divider flexItem sx={{ position: 'relative', left: '-24px', width: 'calc(100% + 24px)' }} />
        <Grid item container direction="row" alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item>
            <Typography variant="body1" sx={{ fontWeight: 600, mb: 1 }}>
              Operating cost
            </Typography>
            <Typography
              variant="body1"
              sx={{
                fontSize: 14,
                mb: 2,
                color: (t) => (t.palette.mode === 'light' ? t.palette.nym.text.muted : 'text.primary'),
              }}
            >
              Changes to cost will be applied in the next interval.
            </Typography>
          </Grid>
          <Grid item xs={12} md={6}>
            <CurrencyFormField
              required
              fullWidth
              label="Operating cost"
              onChanged={(newValue) => {
                setValue('operatorCost', newValue, { shouldValidate: true, shouldDirty: true });
              }}
              validationError={errors.operatorCost?.amount?.message}
              denom={clientDetails?.display_mix_denom || 'nym'}
              initialValue={defaultValues.operatorCost.amount}
            />
            {pendingUpdates && (
              <FormHelperText>
                Your last change to{' '}
                <Typography variant="caption" fontWeight="bold">
                  {pendingUpdates.interval_operating_cost.amount}{' '}
                  {pendingUpdates?.interval_operating_cost.denom.toUpperCase()}{' '}
                </Typography>
                will be applied in the next interval
              </FormHelperText>
            )}
          </Grid>
        </Grid>
        <Divider flexItem sx={{ position: 'relative', left: '-24px', width: 'calc(100% + 24px)' }} />
        <Grid container justifyContent="end">
          <Button
            size="large"
            variant="contained"
            disabled={isSubmitting || !isDirty || !isValid}
            onClick={handleSubmit((data) => {
              getFee(bondedNode.proxy ? simulateVestingUpdateMixnodeCostParams : simulateUpdateMixnodeCostParams, {
                profit_margin_percent: (+data.profitMargin / 100).toString(),
                interval_operating_cost: data.operatorCost,
              });
            })}
            type="submit"
            sx={{ m: 3 }}
          >
            Submit changes to the blockchain
          </Button>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes will take place
        in the next interval"
        okLabel="Close"
        hideCloseIcon
        displayInfoIcon
        onOk={async () => {
          await setOpenConfirmationModal(false);
        }}
        buttonFullWidth
        sx={{
          width: '320px',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
        }}
        headerStyles={{
          width: '100%',
          mb: 1,
          textAlign: 'center',
          color: theme.palette.nym.nymWallet.text.blue,
          fontSize: 16,
        }}
        subHeaderStyles={{
          m: 0,
        }}
      />
    </Grid>
  );
};
