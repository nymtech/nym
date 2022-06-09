import React, { useContext, useEffect } from 'react';
import { Box, Button, Checkbox, CircularProgress, FormControl, FormControlLabel, Grid, TextField } from '@mui/material';
import { yupResolver } from '@hookform/resolvers/yup';
import { useForm } from 'react-hook-form';
import {
  Gateway,
  MixNode,
  EnumNodeType,
  MajorCurrencyAmount,
  CurrencyDenom,
  TransactionExecuteResult,
} from '@nymproject/types';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { TBondArgs } from 'src/types';
import { checkHasEnoughFunds, checkHasEnoughLockedTokens } from 'src/utils';
import { NodeTypeSelector } from '../../components/NodeTypeSelector';
import { bond, vestingBond } from '../../requests';
import { validationSchema } from './validationSchema';
import { AppContext } from '../../context/main';
import { Fee, TokenPoolSelector } from '../../components';

type TBondFormFields = {
  withAdvancedOptions: boolean;
  nodeType: EnumNodeType;
  tokenPool: string;
  ownerSignature: string;
  identityKey: string;
  sphinxKey: string;
  profitMarginPercent: number;
  amount: MajorCurrencyAmount;
  host: string;
  version: string;
  location?: string;
  mixPort: number;
  verlocPort: number;
  clientsPort: number;
  httpApiPort: number;
};

const defaultValues = {
  withAdvancedOptions: false,
  nodeType: EnumNodeType.mixnode,
  tokenPool: 'balance',
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  amount: { amount: '', denom: 'NYM' as CurrencyDenom },
  host: '',
  version: '',
  profitMarginPercent: 10,
  location: undefined,
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
  clientsPort: 9000,
};

const formatData = (data: TBondFormFields): MixNode | Gateway => {
  const payload: { [key: string]: any } = {
    identity_key: data.identityKey,
    sphinx_key: data.sphinxKey,
    host: data.host,
    version: data.version,
    mix_port: data.mixPort,
    profit_margin_percent: data.profitMarginPercent,
  };

  if (data.nodeType === EnumNodeType.mixnode) {
    payload.verloc_port = data.verlocPort;
    payload.http_api_port = data.httpApiPort;
    return payload as MixNode;
  }
  payload.clients_port = data.clientsPort;
  payload.location = data.location;
  return payload as Gateway;
};

export const BondForm = ({
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
    watch,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<TBondFormFields>({
    resolver: yupResolver(validationSchema),
    defaultValues,
  });

  const { userBalance, clientDetails } = useContext(AppContext);

  useEffect(() => {
    reset();
  }, [clientDetails]);

  const watchNodeType = watch('nodeType', defaultValues.nodeType);
  const watchAdvancedOptions = watch('withAdvancedOptions', defaultValues.withAdvancedOptions);

  const onSubmit = async (data: TBondFormFields, cb: (data: TBondArgs) => Promise<TransactionExecuteResult>) => {
    if (data.tokenPool === 'balance' && !(await checkHasEnoughFunds(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough funds in wallet' });
    }

    if (data.tokenPool === 'locked' && !(await checkHasEnoughLockedTokens(data.amount.amount || ''))) {
      return setError('amount.amount', { message: 'Not enough locked tokens' });
    }

    const formattedData = formatData(data);

    return cb({
      type: data.nodeType,
      ownerSignature: data.ownerSignature,
      [data.nodeType]: formattedData,
      pledge: data.amount,
    } as TBondArgs)
      .then(async () => {
        if (data.tokenPool === 'balance') {
          await userBalance.fetchBalance();
        } else {
          await userBalance.fetchTokenAllocation();
        }
        onSuccess({ address: data.identityKey, amount: data.amount.amount });
      })
      .catch((e) => {
        onError(e);
      });
  };

  return (
    <FormControl fullWidth>
      <Box sx={{ p: 3 }}>
        <Grid container spacing={3}>
          <Grid container item justifyContent="space-between">
            <Grid item>
              <NodeTypeSelector
                nodeType={watchNodeType}
                setNodeType={(nodeType) => {
                  setValue('nodeType', nodeType);
                  if (nodeType === EnumNodeType.mixnode) setValue('location', undefined);
                }}
                disabled={disabled}
              />
            </Grid>
          </Grid>
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
              disabled={disabled}
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
              disabled={disabled}
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
              disabled={disabled}
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

          {watchNodeType === EnumNodeType.mixnode && (
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
                disabled={disabled}
              />
            </Grid>
          )}

          {/* if it's a gateway - get location */}
          {watchNodeType === EnumNodeType.gateway && (
            <Grid item xs={6}>
              <TextField
                {...register('location')}
                variant="outlined"
                required
                id="location"
                name="location"
                label="Location"
                fullWidth
                error={!!errors.location}
                helperText={errors.location?.message}
                disabled={disabled}
              />
            </Grid>
          )}

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
              disabled={disabled}
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
              disabled={disabled}
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
                      setValue('clientsPort', defaultValues.clientsPort, {
                        shouldValidate: true,
                      });
                      setValue('verlocPort', defaultValues.verlocPort, {
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
                  disabled={disabled}
                />
              </Grid>
              {watchNodeType === EnumNodeType.mixnode ? (
                <>
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
                      disabled={disabled}
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
                      disabled={disabled}
                    />
                  </Grid>
                </>
              ) : (
                <Grid item xs={12} sm={4}>
                  <TextField
                    {...register('clientsPort', { valueAsNumber: true })}
                    variant="outlined"
                    id="clientsPort"
                    name="clientsPort"
                    label="client WS API Port"
                    fullWidth
                    error={!!errors.clientsPort}
                    helperText={errors.clientsPort?.message && 'A valid port value is required'}
                    disabled={disabled}
                  />
                </Grid>
              )}
            </>
          )}
          <Grid item xs={12}>
            {!disabled ? <Fee feeType={EnumNodeType.mixnode ? 'BondMixnode' : 'BondGateway'} /> : <div />}
          </Grid>
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
          onClick={handleSubmit((data) => onSubmit(data, data.tokenPool === 'balance' ? bond : vestingBond))}
          endIcon={isSubmitting && <CircularProgress size={20} />}
          size="large"
        >
          Bond
        </Button>
      </Box>
    </FormControl>
  );
};
