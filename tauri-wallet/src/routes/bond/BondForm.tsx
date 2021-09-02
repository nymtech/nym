import React, { useState } from 'react'
import {
  Button,
  Checkbox,
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

type TBondNodeFormProps = {
  // minimumBond: Coin
  // onSubmit: (values: BondingInformation) => void
}

type TBondFormFields = {
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

const defaultPorts = {
  mixPort: 1789,
  verlocPort: 1790,
  httpApiPort: 8000,
  clientsPort: 9000,
}

const defaultValues = {
  nodeType: EnumNodeType.Mixnode,
  identityKey: '',
  sphinxKey: '',
  amount: '',
  host: '',
  version: '',
  location: undefined,
  ...defaultPorts,
}

export const BondNodeForm = () => {
  const [advancedShown, setAdvancedShown] = React.useState(false)

  const {
    reset,
    register,
    handleSubmit,
    setValue,
    watch,
    formState: { errors },
  } = useForm<TBondFormFields>({
    resolver: yupResolver(validationSchema),
    defaultValues,
  })

  const watchNodeType = watch('nodeType', EnumNodeType.Mixnode)

  const onSubmit = (data: TBondFormFields) => console.log(data)

  const theme: Theme = useTheme()

  return (
    <form>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={watchNodeType}
              setNodeType={(nodeType) => {
                setValue('nodeType', nodeType)
                // reset(
                //   {
                //     // location:
                //     //   nodeType === EnumNodeType.Mixnode ? undefined : '',
                //     ...defaultPorts,
                //   },
                //   {
                //     keepErrors: true,
                //     keepDirty: true,
                //     keepValues: true,
                //   }
                // )
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
            {watchNodeType === EnumNodeType.Gateway && (
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
                  checked={advancedShown}
                  onChange={() => {
                    setAdvancedShown((shown) => {
                      if (shown) {
                        reset(defaultPorts, {
                          keepErrors: true,
                          keepDirty: true,
                          keepValues: true,
                        })
                      }
                      return !shown
                    })
                  }}
                />
              }
              label="Use advanced options"
            />
          </Grid>

          {advancedShown && (
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
              {watchNodeType === EnumNodeType.Mixnode ? (
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
          disabled={Object.keys(errors).length > 0}
          variant="contained"
          color="primary"
          type="submit"
          size="large"
          disableElevation
          onClick={handleSubmit(onSubmit)}
        >
          Bond
        </Button>
      </div>
    </form>
  )
}
