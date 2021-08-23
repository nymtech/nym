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
import { EnumNodeType } from '../../types/global'
import { useTheme } from '@material-ui/styles'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'

type TBondNodeFormProps = {
  // minimumBond: Coin
  // onSubmit: (values: BondingInformation) => void
}

export const BondNodeForm = () => {
  const [advancedShown, setAdvancedShown] = React.useState(false)
  const [nodeType, setNodeType] = useState(EnumNodeType.Mixnode)

  const theme: Theme = useTheme()

  return (
    <form>
      <div style={{ padding: theme.spacing(3) }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={nodeType}
              setNodeType={(nodeType) => setNodeType(nodeType)}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              variant="outlined"
              required
              id="identityKey"
              name="identityKey"
              label="Identity key"
              fullWidth
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              variant="outlined"
              required
              id="sphinxKey"
              name="sphinxKey"
              label="Sphinx key"
              fullWidth
            />
          </Grid>
          <Grid item xs={12} sm={9}>
            <TextField
              variant="outlined"
              required
              id="amount"
              name="amount"
              label="Amount to bond"
              fullWidth
              InputProps={{
                endAdornment: (
                  <InputAdornment position="end">punks</InputAdornment>
                ),
              }}
            />
          </Grid>

          <Grid item xs={12} sm={6}>
            <TextField
              variant="outlined"
              required
              id="host"
              name="host"
              label="Host"
              fullWidth
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
              variant="outlined"
              required
              id="version"
              name="version"
              label="Version"
              fullWidth
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
        >
          Bond
        </Button>
      </div>
    </form>
  )
}
