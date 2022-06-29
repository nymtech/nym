import React, { useContext, useEffect } from 'react';
import { yupResolver } from '@hookform/resolvers/yup';
import { Box, Button, Checkbox, CircularProgress, FormControl, FormControlLabel, Grid, TextField } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, MajorCurrencyAmount, TransactionExecuteResult } from '@nymproject/types';
import { useForm } from 'react-hook-form';
import { useGetFee } from 'src/hooks/useGetFee';
import { TBondMixNodeArgs } from 'src/types';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from 'src/utils';
import { TokenPoolSelector } from '../../../components';
import { AppContext } from '../../../context/main';
import { bondMixNode, simulateBondMixnode, simulateVestingBondMixnode, vestingBondMixNode } from '../../../requests';
import { mixnodeValidationSchema } from '../validationSchema';
import { ConfirmationModal } from './ConfirmationModal';
import { dataDir } from '@tauri-apps/api/path';
import { LoadingModal } from 'src/components/Modals/LoadingModal';

type TBondFormFields = {
  withAdvancedOptions: boolean;
  tokenPool: string;
  ownerSignature: string;
  identityKey: string;
  sphinxKey: string;
  profitMarginPercent: number;
  amount: MajorCurrencyAmount;
  host: string;
  version: string;
  mixPort: number;
  verlocPort: number;
  httpApiPort: number;
};

const defaultValues = {
  withAdvancedOptions: false,
  tokenPool: 'balance',
  identityKey: 'FTt1HD8ogUdTeqqzX41j3gxaw7t4VB5kMACgAt8nBFTX',
  sphinxKey: 'JAwi4R5DcpaKndsydRqyTbQQZUrK5smBXT6RHiM8Tcqs',
  ownerSignature: 'GcNpA7KWrKHzmDQQNZdms7f9dqrDnC9Z4NEMhtxAayqzhEAX7Jf5r7PcDztbqmrKnVonJNWm58aZZbVmkYTTcda',
  amount: { amount: '100', denom: 'NYM' as CurrencyDenom },
  host: '1.1.1.1',
  version: '1.12.1',
  profitMarginPercent: 10,
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
};

export const MixnodeForm = ({
  disabled,
  onError,
  onSuccess,
}: {
  disabled: boolean;
  onError: (message?: string) => void;
  onSuccess: (details: { address: string; amount: string }) => void;
}) => {
  const {
    register,
    handleSubmit,
    setValue,
    getValues,
    watch,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<TBondFormFields>({
    resolver: yupResolver(mixnodeValidationSchema),
    defaultValues,
  });

  const { userBalance, clientDetails } = useContext(AppContext);

  const { fee, getFee, resetFeeState } = useGetFee();

  useEffect(() => {
    reset();
  }, [clientDetails]);

  const watchAdvancedOptions = watch('withAdvancedOptions', defaultValues.withAdvancedOptions);

  const handleValidateAndGetFee = async (
    data: TBondFormFields,
    cb: (data: TBondMixNodeArgs) => Promise<TransactionExecuteResult>,
  ) => {
    if (data.tokenPool === 'balance' && !(await checkHasEnoughFunds(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough funds in wallet' });
    }

    if (data.tokenPool === 'locked' && !(await checkHasEnoughLockedTokens(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough locked tokens' });
    }

    try {
      await getFee(data.tokenPool === 'locked' ? simulateVestingBondMixnode : simulateBondMixnode, {
        ownerSignature: data.ownerSignature,
        mixnode: {
          identity_key: data.identityKey,
          sphinx_key: data.sphinxKey,
          host: data.host,
          version: data.version,
          mix_port: data.mixPort,
          profit_margin_percent: data.profitMarginPercent,
          verloc_port: data.verlocPort,
          http_api_port: data.httpApiPort,
        },
        pledge: data.amount,
      });
    } catch (e) {
      onError(e as string);
    }
  };

  const onSubmit = async (data: TBondFormFields) => {
    const payload = {
      ownerSignature: data.ownerSignature,
      mixnode: {
        identity_key: data.identityKey,
        sphinx_key: data.sphinxKey,
        host: data.host,
        version: data.version,
        mix_port: data.mixPort,
        profit_margin_percent: data.profitMarginPercent,
        verloc_port: data.verlocPort,
        http_api_port: data.httpApiPort,
      },
      pledge: data.amount,
      fee: fee?.fee,
    };
    try {
      if (data.tokenPool === 'balance') {
        await bondMixNode(payload);
        await userBalance.fetchBalance();
      }

      if (data.tokenPool === 'locked') {
        await vestingBondMixNode(payload);
        await userBalance.fetchTokenAllocation();
      }

      onSuccess({ address: payload.mixnode.identity_key, amount: payload.pledge.amount });
    } catch (e) {
      onError(e as string);
    }
  };

  return (
    <>
      {isSubmitting && <LoadingModal />}

      {fee && !isSubmitting && (
        <ConfirmationModal
          identity={getValues('identityKey')}
          amount={getValues('amount')}
          fee={fee}
          onPrev={resetFeeState}
          onConfirm={handleSubmit(onSubmit)}
        />
      )}

      <FormControl fullWidth>
        <Box>
          <Grid container spacing={3}>
            <Grid container item justifyContent="space-between"></Grid>
            <Grid item xs={12}>
              <TextField
                {...register('identityKey')}
                variant="outlined"
                required
                id="identityKey"
                name="identityKey"
                label="Identity key"
                fullWidth
                error={!!errors.identityKey}
                helperText={errors.identityKey?.message}
                disabled={isSubmitting}
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                {...register('sphinxKey')}
                variant="outlined"
                required
                id="sphinxKey"
                name="sphinxKey"
                label="Sphinx key"
                error={!!errors.sphinxKey}
                helperText={errors.sphinxKey?.message}
                fullWidth
                disabled={isSubmitting}
              />
            </Grid>

            <Grid item xs={12} sm={12}>
              <TextField
                {...register('ownerSignature')}
                variant="outlined"
                required
                id="ownerSignature"
                name="ownerSignature"
                label="Owner signature"
                fullWidth
                error={!!errors.ownerSignature}
                helperText={errors.ownerSignature?.message}
                disabled={isSubmitting}
              />
            </Grid>

            {userBalance.originalVesting && (
              <Grid item xs={12} sm={6}>
                <TokenPoolSelector onSelect={(pool) => setValue('tokenPool', pool)} disabled={disabled} />
              </Grid>
            )}
            <Grid item xs={12} sm={6}>
              <CurrencyFormField
                showCoinMark
                required
                fullWidth
                label="Amount"
                onChanged={(val) => setValue('amount', val, { shouldValidate: true })}
                denom={clientDetails?.denom}
                validationError={errors.amount?.amount?.message}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                {...register('profitMarginPercent')}
                variant="outlined"
                required
                id="profitMarginPercent"
                name="profitMarginPercent"
                label="Profit percentage"
                fullWidth
                error={!!errors.profitMarginPercent}
                helperText={errors.profitMarginPercent ? errors.profitMarginPercent.message : 'Default is 10%'}
                disabled={isSubmitting}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                {...register('host')}
                variant="outlined"
                required
                id="host"
                name="host"
                label="Host"
                fullWidth
                error={!!errors.host}
                helperText={errors.host?.message}
                disabled={isSubmitting}
              />
            </Grid>

            <Grid item xs={12} sm={6}>
              <TextField
                {...register('version')}
                variant="outlined"
                required
                id="version"
                name="version"
                label="Version"
                fullWidth
                error={!!errors.version}
                helperText={errors.version?.message}
                disabled={isSubmitting}
              />
            </Grid>

            <Grid item xs={12}>
              <FormControlLabel
                control={
                  <Checkbox
                    checked={watchAdvancedOptions}
                    onChange={() => {
                      if (watchAdvancedOptions) {
                        setValue('mixPort', defaultValues.mixPort, {
                          shouldValidate: true,
                        });
                        setValue('httpApiPort', defaultValues.httpApiPort, {
                          shouldValidate: true,
                        });
                        setValue('withAdvancedOptions', false);
                      } else {
                        setValue('withAdvancedOptions', true);
                      }
                    }}
                  />
                }
                label="Use advanced options"
              />
            </Grid>
            {watchAdvancedOptions && (
              <>
                <Grid item xs={12} sm={4}>
                  <TextField
                    {...register('mixPort', { valueAsNumber: true })}
                    variant="outlined"
                    id="mixPort"
                    name="mixPort"
                    label="Mix Port"
                    fullWidth
                    error={!!errors.mixPort}
                    helperText={errors.mixPort?.message && 'A valid port value is required'}
                    disabled={isSubmitting}
                  />
                </Grid>

                <Grid item xs={12} sm={4}>
                  <TextField
                    {...register('verlocPort', { valueAsNumber: true })}
                    variant="outlined"
                    id="verlocPort"
                    name="verlocPort"
                    label="Verloc Port"
                    fullWidth
                    error={!!errors.verlocPort}
                    helperText={errors.verlocPort?.message && 'A valid port value is required'}
                    disabled={isSubmitting}
                  />
                </Grid>

                <Grid item xs={12} sm={4}>
                  <TextField
                    {...register('httpApiPort', { valueAsNumber: true })}
                    variant="outlined"
                    id="httpApiPort"
                    name="httpApiPort"
                    label="HTTP API Port"
                    fullWidth
                    error={!!errors.httpApiPort}
                    helperText={errors.httpApiPort?.message && 'A valid port value is required'}
                    disabled={isSubmitting}
                  />
                </Grid>
              </>
            )}
          </Grid>
        </Box>
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'flex-end',
            padding: 3,
            pt: 0,
          }}
        >
          <Button
            disabled={isSubmitting || disabled}
            variant="contained"
            color="primary"
            type="submit"
            data-testid="submit-button"
            disableElevation
            onClick={handleSubmit((data) =>
              handleValidateAndGetFee(data, data.tokenPool === 'balance' ? bondMixNode : vestingBondMixNode),
            )}
            endIcon={isSubmitting && <CircularProgress size={20} />}
            size="large"
          >
            Bond
          </Button>
        </Box>
      </FormControl>
    </>
  );
};
