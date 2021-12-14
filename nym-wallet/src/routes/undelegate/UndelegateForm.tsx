import React, { useContext, useEffect } from 'react'
import { useForm, Controller } from 'react-hook-form'
import {
  Box,
  Alert,
  Autocomplete,
  Button,
  CircularProgress,
  FormControl,
  Grid,
  TextField,
  Typography,
} from '@mui/material'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { EnumNodeType, TFee } from '../../types'
import { ClientContext } from '../../context/main'
import { undelegate } from '../../requests'
import { TDelegations } from '.'

type TFormData = {
  nodeType: EnumNodeType
  identity: string
}

const defaultValues = {
  nodeType: EnumNodeType.mixnode,
  identity: '',
}

export const UndelegateForm = ({
  fees,
  delegations,
  onError,
  onSuccess,
}: {
  fees: TFee
  delegations: TDelegations
  onError: (message?: string) => void
  onSuccess: (message?: string) => void
}) => {
  const {
    control,
    handleSubmit,
    setValue,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<TFormData>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  })
  const watchNodeType = watch('nodeType')

  useEffect(() => {
    setValue('identity', '')
  }, [watchNodeType])

  const { userBalance } = useContext(ClientContext)

  const onSubmit = async (data: TFormData) => {
    await undelegate({
      type: data.nodeType,
      identity: data.identity,
    })
      .then(async (res) => {
        onSuccess(`Successfully undelegated from ${res.target_address}`)
        userBalance.fetchBalance()
      })
      .catch((e) => onError(e))
  }

  return (
    <FormControl fullWidth>
      <Box sx={{ p: 4 }}>
        <Grid container spacing={3} direction="column">
          <Grid item xs={12}>
            <Controller
              control={control}
              name="identity"
              render={({ field }) => (
                <Autocomplete
                  value={field.value}
                  onChange={(_, value) => setValue('identity', value || '')}
                  options={watchNodeType === EnumNodeType.mixnode ? delegations.mixnodes.delegated_nodes : []}
                  renderInput={(params) => (
                    <TextField
                      {...params}
                      required
                      variant="outlined"
                      id="identity"
                      name="identity"
                      label="Mixnode identity"
                      error={!!errors.identity}
                      helperText={errors.identity?.message}
                      fullWidth
                    />
                  )}
                />
              )}
            />
          </Grid>
          <Grid item xs={12}>
            <Typography sx={{ color: 'nym.info' }}>Fee for this transaction: {fees.mixnode.amount} punk</Typography>
          </Grid>
        </Grid>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'flex-end',
          borderTop: (theme) => `1px solid ${theme.palette.grey[200]}`,
          bgcolor: 'grey.100',
          p: 2,
        }}
      >
        <Button
          onClick={handleSubmit(onSubmit)}
          variant="contained"
          color="primary"
          type="submit"
          data-testid="submit-button"
          disableElevation
          disabled={isSubmitting}
          endIcon={isSubmitting && <CircularProgress size={20} />}
        >
          Undelegate stake
        </Button>
      </Box>
    </FormControl>
  )
}
