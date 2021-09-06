import React from 'react'
import {
  Button,
  Checkbox,
  CircularProgress,
  FormControl,
  FormControlLabel,
  Grid,
  InputAdornment,
  TextField,
  Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { useForm } from 'react-hook-form'
import { yupResolver } from '@hookform/resolvers/yup'
import { EnumNodeType } from '../../types/global'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { validationSchema } from './validationSchema'
import { Gateway, MixNode } from '../../types'
import { invoke } from '@tauri-apps/api'

type TBondFormFields = {
  withAdvancedOptions: boolean
  nodeType: EnumNodeType
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
    amount: data.amount,
    nodeType: data.nodeType,
  }

  if (data.nodeType === EnumNodeType.mixnode) {
    payload.verloc_port = data.verlocPort
    payload.http_api_port = data.httpApiPort
    return payload as MixNode & { amount: number; nodeType: EnumNodeType }
  }

  if (data.nodeType == EnumNodeType.gateway) {
    payload.clients_port = data.clientsPort
    payload.location = data.location
    return payload as Gateway & { amount: number; nodeType: EnumNodeType }
  }
}

export const BondForm = ({
  onError,
  onSuccess,
}: {
  onError: (message?: string) => void
  onSuccess: (message?: string) => void
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

  const watchNodeType = watch('nodeType', defaultValues.nodeType)
  const watchAdvancedOptions = watch(
    'withAdvancedOptions',
    defaultValues.withAdvancedOptions
  )

  const onSubmit = async (data: TBondFormFields) => {
    const formattedData = formatData(data)
    await invoke(`bond_${data.nodeType}`, {
      [data.nodeType]: formattedData,
      bond: { amount: formattedData?.amount, denom: 'punk' },
    })
      .then((res: any) => {
        onSuccess(res)
      })
      .catch((e) => {
        onError(e)
      })
  }

  const theme: Theme = useTheme()

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={watchNodeType}
              setNodeType={(nodeType) => {
                setValue('nodeType', nodeType)
                if (nodeType === EnumNodeType.mixnode)
                  setValue('location', undefined)
              }}
            />
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
            />
          </Grid>
          <Grid item xs={12} sm={9}>
            <TextField
              {...register('amount')}
              variant="outlined"
              required
              id="amount"
              name="amount"
              label="Amount to bond"
              fullWidth
              error={!!errors.amount}
              helperText={errors.amount?.message}
              InputProps={{
                endAdornment: (
                  <InputAdornment position="end">punks</InputAdornment>
                ),
              }}
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
                  helperText={
                    errors.mixPort?.message && 'A valid port value is required'
                  }
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
                      helperText={
                        errors.verlocPort?.message &&
                        'A valid port value is required'
                      }
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
                      helperText={
                        errors.httpApiPort?.message &&
                        'A valid port value is required'
                      }
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
                    helperText={
                      errors.clientsPort?.message &&
                      'A valid port value is required'
                    }
                  />
                </Grid>
              )}
            </>
          )}
        </Grid>
      </div>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: `1px solid ${theme.palette.grey[200]}`,
          background: theme.palette.grey[100],
          padding: theme.spacing(2),
        }}
      >
        <Button
          disabled={isSubmitting}
          variant="contained"
          color="primary"
          type="submit"
          size="large"
          disableElevation
          onClick={handleSubmit(onSubmit)}
          endIcon={isSubmitting && <CircularProgress size={20} />}
        >
          Bond
        </Button>
      </div>
    </FormControl>
  )
}
