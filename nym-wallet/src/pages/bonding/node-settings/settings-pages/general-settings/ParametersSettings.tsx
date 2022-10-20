import React, { useContext, useEffect, useState } from 'react';
import { useForm } from 'react-hook-form';
import { yupResolver } from '@hookform/resolvers/yup';
import {
  Button,
  Divider,
  Typography,
  TextField,
  InputAdornment,
  Grid,
  CircularProgress,
  Box,
  FormHelperText,
} from '@mui/material';
import { useTheme } from '@mui/material/styles';
import { CurrencyDenom, MixNodeCostParams } from '@nymproject/types';
import { add, format, fromUnixTime } from 'date-fns';
import { isMixnode } from 'src/types';
import { getCurrentInterval, getPendingIntervalEvents, updateMixnodeCostParams } from 'src/requests';
import { TBondedMixnode, TBondedGateway } from 'src/context/bonding';
import { SimpleModal } from 'src/components/Modals/SimpleModal';
import { bondedNodeParametersValidationSchema } from 'src/components/Bonding/forms/mixnodeValidationSchema';
import { Console } from 'src/utils/console';
import { Alert } from 'src/components/Alert';
import { ChangeMixCostParams } from 'src/pages/bonding/types';
import { AppContext } from 'src/context';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';

export const ParametersSettings = ({ bondedNode }: { bondedNode: TBondedMixnode }): JSX.Element => {
  const [openConfirmationModal, setOpenConfirmationModal] = useState<boolean>(false);
  const [intervalTime, setIntervalTime] = useState<string>();
  const [nextEpoch, setNextEpoch] = useState<string>();
  const [pendingUpdates, setPendingUpdates] = useState<MixNodeCostParams>();
  const { clientDetails } = useContext(AppContext);
  const theme = useTheme();

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

  const getIntervalAsDate = async () => {
    const interval = await getCurrentInterval();
    const secondsToNextInterval =
      Number(interval.epochs_in_interval - interval.current_epoch_id) * Number(interval.epoch_length_seconds);

    setIntervalTime(
      format(
        add(new Date(), {
          seconds: secondsToNextInterval,
        }),
        'MM/dd/yyyy HH:mm',
      ),
    );
    setNextEpoch(
      format(
        add(fromUnixTime(Number(interval.current_epoch_start_unix)), {
          seconds: Number(interval.epoch_length_seconds),
        }),
        'HH:mm',
      ),
    );
  };

  const getPendingEvents = async () => {
    const events = await getPendingIntervalEvents();
    const latestEvent = events.reverse().find((evt) => 'ChangeMixCostParams' in evt.event) as unknown as
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
    getIntervalAsDate();
    getPendingEvents();
  }, []);

  const onSubmit = async (data: { operatorCost: { amount: string; denom: CurrencyDenom }; profitMargin: string }) => {
    if (data.operatorCost && data.profitMargin) {
      const MixNodeCostParams = {
        profit_margin_percent: (+data.profitMargin / 100).toString(),
        interval_operating_cost: {
          amount: data.operatorCost.amount,
          denom: data.operatorCost.denom,
        },
      };
      try {
        await updateMixnodeCostParams(MixNodeCostParams);
        await getPendingEvents();
        reset();
        setOpenConfirmationModal(true);
      } catch (error) {
        Console.error(error);
      }
    }
  };

  return (
    <Grid container xs item>
      <Alert
        title={
          <>
            <Box component="span" sx={{ fontWeight: 600, mr: 2 }}>
              {`Next epoch ${nextEpoch}`}
            </Box>
            <Box component="span" sx={{ fontWeight: 600 }}>{`Next interval: ${intervalTime}`}</Box>
          </>
        }
      />
      <Grid container direction="column">
        <Grid item container alignItems="left" justifyContent="space-between" padding={3} spacing={1}>
          <Grid item>
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
            <Grid item xs={12} xl={6}>
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
        <Divider flexItem />
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
          <Grid spacing={3} container item alignItems="center" xs={12} md={6}>
            <Grid item width={1}>
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
        </Grid>
        <Divider flexItem />
        <Grid container justifyContent="end">
          <Button
            size="large"
            variant="contained"
            disabled={isSubmitting || !isDirty || !isValid}
            onClick={handleSubmit(onSubmit)}
            type="submit"
            sx={{ m: 3, width: '320px' }}
            endIcon={isSubmitting && <CircularProgress size={20} />}
          >
            Save all display changes
          </Button>
        </Grid>
      </Grid>
      <SimpleModal
        open={openConfirmationModal}
        header="Your changes will take place
        in the next interval"
        okLabel="close"
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
          textTransform: 'capitalize',
        }}
        subHeaderStyles={{
          m: 0,
        }}
      />
    </Grid>
  );
};
