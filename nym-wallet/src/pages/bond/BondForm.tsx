import React, { useContext } from 'react'
import {
  Box,
  Button,
  Checkbox,
  CircularProgress,
  FormControl,
  FormControlLabel,
  Grid,
  InputAdornment,
  TextField,
  Typography,
} from '@mui/material'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm } from 'react-hook-form'
import { EnumNodeType } from '../../types/global'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { bond, majorToMinor } from '../../requests'
import { validationSchema } from './validationSchema'
import { Gateway, MixNode } from '../../types'
import { ClientContext } from '../../context/main'
import { Fee } from '../../components'

type TBondFormFields = {
  withAdvancedOptions: boolean
  nodeType: EnumNodeType
  ownerSignature: string
  identityKey: string
  sphinxKey: string
  profitMarginPercent: number
  amount: string
  host: string
  version: string
  location?: string
  mixPort: number
  verlocPort: number
  clientsPort: number
  httpApiPort: number
}

const defaultValues = {
  withAdvancedOptions: false,
  nodeType: EnumNodeType.mixnode,
  identityKey: '',
  sphinxKey: '',
  ownerSignature: '',
  amount: '',
  host: '',
  version: '',
  profitMarginPercent: 10,
  location: undefined,
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
  clientsPort: 9000,
}

const formatData = (data: TBondFormFields) => {
  const payload: { [key: string]: any } = {
    identity_key: data.identityKey,
    sphinx_key: data.sphinxKey,
    host: data.host,
    version: data.version,
    mix_port: data.mixPort,
    profit_margin_percent: data.profitMarginPercent,
  }

  if (data.nodeType === EnumNodeType.mixnode) {
    payload.verloc_port = data.verlocPort
    payload.http_api_port = data.httpApiPort
    return payload as MixNode
  } else {
    payload.clients_port = data.clientsPort
    payload.location = data.location
    return payload as Gateway
  }
}

export const BondForm = ({
  disabled,
  onError,
  onSuccess,
}: {
  disabled: boolean
  onError: (message?: string) => void
  onSuccess: (details: { address: string; amount: string }) => void
}) => {
  const {
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<TBondFormFields>({
    resolver: yupResolver(validationSchema),
    defaultValues,
  })

  const { userBalance, currency, getBondDetails } = useContext(ClientContext)

  const watchNodeType = watch('nodeType', defaultValues.nodeType)
  const watchAdvancedOptions = watch('withAdvancedOptions', defaultValues.withAdvancedOptions)

  const onSubmit = async (data: TBondFormFields) => {
    const formattedData = formatData(data)
    const pledge = await majorToMinor(data.amount)

    await bond({ type: data.nodeType, ownerSignature: data.ownerSignature, data: formattedData, pledge })
      .then(async () => {
        await getBondDetails()
        userBalance.fetchBalance()
        onSuccess({ address: data.identityKey, amount: data.amount })
      })
      .catch((e) => {
        onError(e)
      })
  }

  return (
    <FormControl fullWidth>
      <Box sx={{ p: 3 }}>
        <Grid container spacing={3}>
          <Grid container item justifyContent="space-between">
            <Grid item>
              <NodeTypeSelector
                nodeType={watchNodeType}
                setNodeType={(nodeType) => {
                  setValue('nodeType', nodeType)
                  if (nodeType === EnumNodeType.mixnode) setValue('location', undefined)
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

          <Grid item xs={12} sm={6}>
            <TextField
              {...register('amount')}
              variant="outlined"
              required
              id="amount"
              name="amount"
              label="Amount to pledge"
              fullWidth
              error={!!errors.amount}
              helperText={errors.amount?.message}
              InputProps={{
                endAdornment: <InputAdornment position="end">{currency?.major}</InputAdornment>,
              }}
              disabled={disabled}
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
                      })
                      setValue('clientsPort', defaultValues.clientsPort, {
                        shouldValidate: true,
                      })
                      setValue('verlocPort', defaultValues.verlocPort, {
                        shouldValidate: true,
                      })
                      setValue('httpApiPort', defaultValues.httpApiPort, {
                        shouldValidate: true,
                      })
                      setValue('withAdvancedOptions', false)
                      resizeTo
                    } else {
                      setValue('withAdvancedOptions', true)
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
          <Grid item>
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
          onClick={handleSubmit(onSubmit)}
          endIcon={isSubmitting && <CircularProgress size={20} />}
          size="large"
        >
          Bond
        </Button>
      </Box>
    </FormControl>
  )
}
