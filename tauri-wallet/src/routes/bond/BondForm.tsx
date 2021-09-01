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
import * as Yup from 'yup'
import { EnumNodeType } from '../../types/global'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import {
  isValidHostname,
  validateAmount,
  validateKey,
  validateVersion,
} from '../../utils'

type TBondNodeFormProps = {
  // minimumBond: Coin
  // onSubmit: (values: BondingInformation) => void
}

type TBondFormFields = {
  identityKey: string
  sphinxKey: string
  amount: string
  host: string
  version: string
}

const validationSchema = Yup.object().shape({
  identityKey: Yup.string()
    .required('An indentity key is required')
    .test('valid-id-key', 'A valid identity key is required', function (value) {
      return validateKey(value || '')
    }),
  sphinxKey: Yup.string()
    .required('A sphinx key is required')
    .test(
      'valid-sphinx-key',
      'A valid sphinx key is required',
      function (value) {
        return validateKey(value || '')
      }
    ),
  amount: Yup.string()
    .required('An amount is required')
    .test(
      'valid-amount',
      'A valid amount is required (min 100 punks)',
      function (value) {
        return validateAmount(value || '', '100000000')
        // minimum amount needs to come from the backend - replace when available
      }
    ),

  host: Yup.string()
    .required('A host is required')
    .test('valid-amount', 'A valid host is required', function (value) {
      return !!value ? isValidHostname(value) : false
    }),
  version: Yup.string()
    .required('A version is required')
    .test('valid-version', 'A valid version is required', function (value) {
      return !!value ? validateVersion(value) : false
    }),
})

export const BondNodeForm = () => {
  const [advancedShown, setAdvancedShown] = React.useState(false)
  const [nodeType, setNodeType] = useState(EnumNodeType.Mixnode)

  const {
    register,
    handleSubmit,
    formState: { errors },
  } = useForm<TBondFormFields>({ resolver: yupResolver(validationSchema) })

  const theme: Theme = useTheme()
  console.log(errors)

  const onSubmit = (data: TBondFormFields) => console.log(data)

  return (
    <form>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={nodeType}
              setNodeType={(nodeType) => setNodeType(nodeType)}
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
            {nodeType === EnumNodeType.Gateway && (
              <TextField
                variant="outlined"
                required
                id="location"
                name="location"
                label="Location"
                fullWidth
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
                  variant="outlined"
                  id="mixPort"
                  name="mixPort"
                  label="Mix Port"
                  fullWidth
                />
              </Grid>
              {nodeType === EnumNodeType.Mixnode ? (
                <>
                  <Grid item xs={12} sm={4}>
                    <TextField
                      variant="outlined"
                      id="verlocPort"
                      name="verlocPort"
                      label="Verloc Port"
                      fullWidth
                    />
                  </Grid>

                  <Grid item xs={12} sm={4}>
                    <TextField
                      variant="outlined"
                      id="httpApiPort"
                      name="httpApiPort"
                      label="HTTP API Port"
                      fullWidth
                    />
                  </Grid>
                </>
              ) : (
                <Grid item xs={12} sm={4}>
                  <TextField
                    variant="outlined"
                    id="clientsPort"
                    name="clientsPort"
                    label="client WS API Port"
                    fullWidth
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
