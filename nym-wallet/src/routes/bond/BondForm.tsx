import React, { useContext } from 'react'
import {
  Alert,
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
import { Coin, Gateway, MixNode } from '../../types'
import { ClientContext } from '../../context/main'
import { checkHasEnoughFunds } from '../../utils'

type TBondFormFields = {
  withAdvancedOptions: boolean
  nodeType: EnumNodeType
  ownerSignature: string,
  identityKey: string
  sphinxKey: string
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
  amount: '',
  host: '',
  version: '',
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
  fees,
  onError,
  onSuccess,
}: {
  disabled: boolean
  fees?: { [EnumNodeType.mixnode]: Coin; [EnumNodeType.gateway]?: Coin }
  onError: (message?: string) => void
  onSuccess: (details: { address: string; amount: string }) => void
}) => {
  const {
    register,
    handleSubmit,
    setValue,
    setError,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<TBondFormFields>({
    resolver: yupResolver(validationSchema),
    defaultValues,
  })

  const { userBalance } = useContext(ClientContext)

  const watchNodeType = watch('nodeType', defaultValues.nodeType)
  const watchAdvancedOptions = watch('withAdvancedOptions', defaultValues.withAdvancedOptions)

  const onSubmit = async (data: TBondFormFields) => {
    const hasEnoughFunds = await checkHasEnoughFunds(data.amount)
    if (!hasEnoughFunds) {
      return setError('amount', { message: 'Not enough funds in wallet' })
    }

    const formattedData = formatData(data)
    const amount = await majorToMinor(data.amount)

    await bond({ type: data.nodeType, data: formattedData, amount })
      .then(() => {
        userBalance.fetchBalance()
        onSuccess({ address: data.identityKey, amount: data.amount })
      })
      .catch((e) => {
        onError(e)
      })
  }

  return (
    <FormControl fullWidth>
      <Box sx={{ padding: [3, 5] }}>
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
          <Grid item xs={12} sm={9}>
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
                endAdornment: <InputAdornment position="end">punk</InputAdornment>,
              }}
              disabled={disabled}
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
              disabled={disabled}
            />
          </Grid>

          {/* if it's a gateway - get location */}
          <Grid item xs={6}>
            {watchNodeType === EnumNodeType.gateway && (
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
            )}
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


          <Grid item xs={12} sm={12}>
            <TextField
                {...register('ownerSignature')}
                variant="outlined"
                required
                id="ownerSignature"
                name="ownerSignature"
                label="Signature on your address"
                fullWidth
                error={!!errors.ownerSignature}
                helperText={errors.ownerSignature?.message}
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
          {fees && (
            <Grid item xs={12}>
              <Typography sx={{ color: 'nym.info' }}>
                {' '}
                {`A bonding fee: ${
                  watchNodeType === EnumNodeType.mixnode ? fees.mixnode.amount : fees?.gateway?.amount
                }`}
              </Typography>
            </Grid>
          )}
        </Grid>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
          bgcolor: 'grey.100',
          padding: 2,
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
        >
          Bond
        </Button>
      </Box>
    </FormControl>
  )
}
