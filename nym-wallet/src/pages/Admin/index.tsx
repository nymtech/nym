import React, { useContext, useEffect, useState } from 'react'
import { useForm } from 'react-hook-form'
import { Backdrop, Box, Button, CircularProgress, FormControl, Grid, Paper, Slide, TextField } from '@mui/material'

import { ClientContext } from '../../context/main'
import { NymCard } from '../../components'
import { getContractParams, setContractParams } from '../../requests'
import { TauriContractStateParams } from '../../types'

export const Admin: React.FC = () => {
  const { showAdmin, handleShowAdmin } = useContext(ClientContext)
  const [isLoading, setIsLoading] = useState(false)
  const [params, setParams] = useState<TauriContractStateParams>()

  const onCancel = () => {
    setParams(undefined)
    setIsLoading(false)
    handleShowAdmin()
  }

  useEffect(() => {
    const requestContractParams = async () => {
      if (showAdmin) {
        setIsLoading(true)
        const params = await getContractParams()
        setParams(params)
        setIsLoading(false)
      }
    }
    requestContractParams()
  }, [showAdmin])

  return (
    <Backdrop open={showAdmin} style={{ zIndex: 2, overflow: 'auto' }}>
      <Slide in={showAdmin}>
        <Paper style={{ margin: 'auto' }}>
          <NymCard title="Admin" subheader="Contract administration" noPadding>
            {isLoading && (
              <Box style={{ display: 'flex', justifyContent: 'center' }}>
                <CircularProgress size={48} />
              </Box>
            )}
            {!isLoading && params && <AdminForm onCancel={onCancel} params={params} />}
          </NymCard>
        </Paper>
      </Slide>
    </Backdrop>
  )
}

const AdminForm: React.FC<{
  params: TauriContractStateParams
  onCancel: () => void
}> = ({ params, onCancel }) => {
  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm({ defaultValues: { ...params } })

  const onSubmit = async (data: TauriContractStateParams) => {
    await setContractParams(data)
    console.log(data)
    onCancel()
  }

  return (
    <FormControl fullWidth>
      <Box sx={{ padding: [3, 5], maxWidth: 700, minWidth: 400 }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <TextField
              {...register('minimum_mixnode_bond')}
              required
              variant="outlined"
              id="minimum_mixnode_bond"
              name="minimum_mixnode_bond"
              label="Minumum mixnode bond"
              fullWidth
              error={!!errors.minimum_mixnode_bond}
              helperText={errors?.minimum_mixnode_bond?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('minimum_gateway_bond')}
              required
              variant="outlined"
              id="minimum_gateway_bond"
              name="minimum_gateway_bond"
              label="Minumum gateway bond"
              fullWidth
              error={!!errors.minimum_gateway_bond}
              helperText={errors?.minimum_gateway_bond?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixnode_bond_reward_rate')}
              required
              variant="outlined"
              id="mixnode_bond_reward_rate"
              name="mixnode_bond_reward_rate"
              label="Mixnode bond reward rate"
              fullWidth
              error={!!errors.mixnode_bond_reward_rate}
              helperText={errors?.mixnode_bond_reward_rate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('gateway_bond_reward_rate')}
              required
              variant="outlined"
              id="gateway_bond_reward_rate"
              name="gateway_bond_reward_rate"
              label="Gateway bond reward rate"
              fullWidth
              error={!!errors.gateway_bond_reward_rate}
              helperText={errors?.gateway_bond_reward_rate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixnode_delegation_reward_rate')}
              required
              variant="outlined"
              id="mixnode_delegation_reward_rate"
              name="mixnode_delegation_reward_rate"
              label="Mixnode Delegation Reward Rate"
              fullWidth
              error={!!errors.mixnode_delegation_reward_rate}
              helperText={errors?.mixnode_delegation_reward_rate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('gateway_delegation_reward_rate')}
              required
              variant="outlined"
              id="gateway_delegation_reward_rate"
              name="gateway_delegation_reward_rate"
              label="Gateway Delegation Reward Rate"
              fullWidth
              error={!!errors.gateway_delegation_reward_rate}
              helperText={errors?.gateway_delegation_reward_rate?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('epoch_length')}
              required
              variant="outlined"
              id="epochLength"
              name="epochLength"
              label="Epoch length (hours)"
              fullWidth
              error={!!errors.epoch_length}
              helperText={errors?.epoch_length?.message}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('mixnode_active_set_size', { valueAsNumber: true })}
              required
              variant="outlined"
              id="mixnode_active_set_size"
              name="mixnode_active_set_size"
              label="Mixnode Active Set Size "
              fullWidth
              error={!!errors.mixnode_active_set_size}
              helperText={errors?.mixnode_active_set_size?.message}
            />
          </Grid>
        </Grid>
      </Box>
      <Grid
        container
        spacing={1}
        justifyContent="flex-end"
        sx={{
          borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
          background: (theme) => theme.palette.grey[100],
          padding: 2,
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
