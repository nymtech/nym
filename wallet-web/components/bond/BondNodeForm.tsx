import React from 'react'
import {
  Button,
  Checkbox,
  FormControlLabel,
  Grid,
  InputAdornment,
  TextField,
  useMediaQuery,
} from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { Coin, nativeToPrintable } from '@nymproject/nym-validator-client'
import { NodeType } from '../../common/node'
import { DENOM } from '../../pages/_app'
import { theme } from '../../lib/theme'
import { BondingInformation } from './NodeBond'
import { useBondForm } from './useBondForm'

type TBondNodeFormProps = {
  type: NodeType
  minimumBond: Coin
  onSubmit: (values: BondingInformation) => void
}

export const BondNodeForm = ({
  type,
  minimumBond,
  onSubmit,
}: TBondNodeFormProps) => {
  const [advancedShown, setAdvancedShown] = React.useState(false)

  const manageForm = useBondForm({ type, minimumBond })

  const matches = useMediaQuery('(min-width:768px)')

  return (
    <form
      onSubmit={(e: React.FormEvent<HTMLFormElement>) => {
        e.preventDefault()
        manageForm.handleSubmit(onSubmit)
      }}
    >
      <Grid container spacing={3}>
        <Grid item xs={12} sm={8}>
          <TextField
            required
            value={manageForm.formData.amount.value}
            onChange={manageForm.handleAmountChange}
            id="amount"
            name="amount"
            label={`Amount to pledge ${
              matches
                ? '(minimum ' + nativeToPrintable(minimumBond.amount) + ')'
                : ''
            }`}
            error={manageForm.formData.amount.isValid === false}
            helperText={
              manageForm.formData.amount.isValid === false
                ? `Enter a valid pledge amount (minimum ${nativeToPrintable(
                    minimumBond.amount
                  )})`
                : ''
            }
            fullWidth
            InputProps={{
              endAdornment: (
                <InputAdornment position="end">{DENOM}</InputAdornment>
              ),
            }}
          />
        </Grid>
        {manageForm.allocationWarning.message && (
          <Grid item>
            <Alert
              severity={manageForm.allocationWarning.error ? 'error' : 'info'}
            >
              {manageForm.allocationWarning.message}
            </Alert>
          </Grid>
        )}
        <Grid item xs={12}>
          <TextField
            value={manageForm.formData.identityKey.value}
            onChange={manageForm.handleIdentityKeyChange}
            error={manageForm.formData.identityKey.isValid === false}
            required
            id="identityKey"
            name="identityKey"
            label="Identity key"
            fullWidth
            {...(manageForm.formData.identityKey.isValid === false
              ? { helperText: 'Enter a valid identity key' }
              : {})}
          />
        </Grid>
        <Grid item xs={12}>
          <TextField
            value={manageForm.formData.sphinxKey.value}
            onChange={manageForm.handleShinxKeyChange}
            error={manageForm.formData.sphinxKey.isValid === false}
            required
            id="sphinxKey"
            name="sphinxKey"
            label="Sphinx key"
            fullWidth
            {...(manageForm.formData.sphinxKey.isValid === false
              ? { helperText: 'Enter a valid sphinx key' }
              : {})}
          />
        </Grid>
        <Grid item xs={12} sm={6}>
          <TextField
            value={manageForm.formData.host.value}
            onChange={manageForm.handleHostChange}
            error={manageForm.formData.host.isValid === false}
            required
            id="host"
            name="host"
            label="Host"
            fullWidth
            {...(manageForm.formData.host.isValid === false
              ? { helperText: 'Enter a valid IP or a hostname (without port)' }
              : {})}
          />
        </Grid>

        {/* if it's a gateway - get location */}
        <Grid item xs={12} sm={6}>
          {type === NodeType.Gateway && (
            <TextField
              value={manageForm.formData.location.value}
              onChange={manageForm.handleLocationChange}
              error={manageForm.formData.location.isValid === false}
              required
              id="location"
              name="location"
              label="Location"
              fullWidth
              {...(manageForm.formData.location.isValid === false
                ? { helperText: 'Enter a valid location of your node' }
                : {})}
            />
          )}
        </Grid>

        <Grid item xs={12} sm={6}>
          <TextField
            value={manageForm.formData.version.value}
            onChange={manageForm.handleVersionChange}
            error={manageForm.formData.version.isValid === false}
            required
            id="version"
            name="version"
            label="Version"
            fullWidth
            {...(manageForm.formData.version.isValid === false
              ? {
                  helperText:
                    'Enter a valid version (min. 0.11.0), like 0.11.0',
                }
              : {})}
          />
        </Grid>

        <Grid item xs={12}>
          <FormControlLabel
            control={
              <Checkbox
                checked={advancedShown}
                onChange={() => {
                  setAdvancedShown((shown) => {
                    if (shown) manageForm.initialisePorts()
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
                value={manageForm.formData.mixPort.value}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  manageForm.handlePortChange('mixPort', e.target.value)
                }
                error={manageForm.formData.mixPort.isValid === false}
                variant="outlined"
                id="mixPort"
                name="mixPort"
                label="Mix Port"
                fullWidth
              />
            </Grid>
            {type === NodeType.Mixnode ? (
              <>
                <Grid item xs={12} sm={4}>
                  <TextField
                    value={manageForm.formData.verlocPort.value}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      manageForm.handlePortChange('verlocPort', e.target.value)
                    }
                    error={manageForm.formData.verlocPort.isValid === false}
                    variant="outlined"
                    id="verlocPort"
                    name="verlocPort"
                    label="Verloc Port"
                    fullWidth
                  />
                </Grid>

                <Grid item xs={12} sm={4}>
                  <TextField
                    value={manageForm.formData.httpApiPort.value}
                    onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                      manageForm.handlePortChange('httpApiPort', e.target.value)
                    }
                    error={manageForm.formData.httpApiPort.isValid === false}
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
                  value={manageForm.formData.clientsPort.value}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                    manageForm.handlePortChange('clientsPort', e.target.value)
                  }
                  error={manageForm.formData.clientsPort.isValid === false}
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

      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          padding: theme.spacing(2),
        }}
      >
        <Button
          variant="contained"
          color="primary"
          type="submit"
          size="large"
          disabled={!manageForm.isValidForm}
        >
          Bond
        </Button>
      </div>
    </form>
  )
}
