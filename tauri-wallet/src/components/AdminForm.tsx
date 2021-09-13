import React, { useContext } from 'react'
import { useForm } from 'react-hook-form'
import {
  Backdrop,
  Button,
  CircularProgress,
  FormControl,
  Grid,
  Paper,
  Slide,
  TextField,
  Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { ClientContext } from '../context/main'
import { NymCard } from '.'

export const Admin: React.FC = () => {
  const { showAdmin, handleShowAdmin } = useContext(ClientContext)

  const onCancel = () => {
    handleShowAdmin()
  }

  return (
    <Backdrop open={showAdmin} style={{ zIndex: 2, overflow: 'auto' }}>
      <Slide in={showAdmin}>
        <Paper style={{ margin: 'auto' }}>
          <NymCard title="Admin" subheader="Contract administration" noPadding>
            <AdminForm onCancel={onCancel} />
          </NymCard>
        </Paper>
      </Slide>
    </Backdrop>
  )
}

const AdminForm: React.FC<{ onCancel: () => void }> = ({ onCancel }) => {
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm()

  const onSubmit = (data: any) => {
    console.log(data)
    onCancel()
  }

  const theme: Theme = useTheme()

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5), maxWidth: 700 }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <TextField
              {...register('minimumMixnodeBond')}
              required
              variant="outlined"
              id="minimumMixnodeBond"
              name="minimumMixnodeBond"
              label="Minumum mixnode bond"
              fullWidth
              error={!!errors.minimumMixnodeBond}
              helperText={errors?.minimumMixnodeBond?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('minimumGatewayBond')}
              required
              variant="outlined"
              id="minimumGatewayBond"
              name="minimumGatewayBond"
              label="Minumum gateway bond"
              fullWidth
              error={!!errors.minimumGatewayBond}
              helperText={errors?.minimumGatewayBond?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixnodeBondRewardRate')}
              required
              variant="outlined"
              id="mixnodeBondRewardRate"
              name="mixnodeBondRewardRate"
              label="Mixnode bond reward rate"
              fullWidth
              error={!!errors.mixnodeBondRewardRate}
              helperText={errors?.mixnodeBondRewardRate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('gatewayBondRewardRate')}
              required
              variant="outlined"
              id="gatewayBondRewardRate"
              name="gatewayBondRewardRate"
              label="Gateway bond reward rate"
              fullWidth
              error={!!errors.gatewayBondRewardRate}
              helperText={errors?.gatewayBondRewardRate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixnodeDelegationRewardRate')}
              required
              variant="outlined"
              id="mixnodeDelegationRewardRate"
              name="mixnodeDelegationRewardRate"
              label="Mixnode Delegation Reward Rate"
              fullWidth
              error={!!errors.mixnodeDelegationRewardRate}
              helperText={errors?.mixnodeDelegationRewardRate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('gatewayDelegationRewardRate')}
              required
              variant="outlined"
              id="gatewayDelegationRewardRate"
              name="gatewayDelegationRewardRate"
              label="Gateway Delegation Reward Rate"
              fullWidth
              error={!!errors.gatewayDelegationRewardRate}
              helperText={errors?.gatewayDelegationRewardRate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('epochLength')}
              required
              variant="outlined"
              id="epochLength"
              name="epochLength"
              label="Epoch length (hours)"
              fullWidth
              error={!!errors.epochLength}
              helperText={errors?.epochLength?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixNodeActiveSetSize')}
              required
              variant="outlined"
              id="mixNodeActiveSetSize"
              name="mixNodeActiveSetSize"
              label="Mixnode Active Set Size "
              fullWidth
              error={!!errors.epochLength}
              helperText={errors?.epochLength?.message}
            />
          </Grid>
        </Grid>
      </div>
      <Grid
        container
        spacing={1}
        justifyContent="flex-end"
        style={{
          borderTop: `1px solid ${theme.palette.grey[200]}`,
          background: theme.palette.grey[100],
          padding: theme.spacing(2),
        }}
      >
        <Grid item>
          <Button onClick={onCancel}>Cancel</Button>
        </Grid>
        <Grid item>
          <Button
            onClick={handleSubmit(onSubmit)}
            disabled={isSubmitting}
            variant="contained"
            color="primary"
            type="submit"
            disableElevation
            endIcon={isSubmitting && <CircularProgress size={20} />}
          >
            Update Contract
          </Button>
        </Grid>
      </Grid>
    </FormControl>
  )
}
