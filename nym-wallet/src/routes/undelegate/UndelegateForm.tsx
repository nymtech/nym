import React, { useContext, useEffect } from 'react'
import { useForm, Controller } from 'react-hook-form'
import {
  Button,
  CircularProgress,
  FormControl,
  Grid,
  TextField,
  Theme,
} from '@material-ui/core'
import { Alert, Autocomplete } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
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

  const { getBalance } = useContext(ClientContext)

  const theme: Theme = useTheme()

  const onSubmit = async (data: TFormData) => {
    await undelegate({
      type: data.nodeType,
      identity: data.identity,
    })
      .then(async (res) => {
        onSuccess(`Successfully undelegated from ${res.target_address}`)
        getBalance.fetchBalance()
      })
      .catch((e) => onError(e))
  }

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3} direction="column">
          <Grid container item xs={12} justifyContent="space-between">
            <Grid item>
              <Alert severity="info" data-testid="fee-amount">
                {`A fee of ${fees.mixnode.amount} PUNK will apply to this transaction`}
              </Alert>
            </Grid>
          </Grid>
          <Grid item xs={12}>
            <Controller
              control={control}
              name="identity"
              render={({ field }) => (
                <Autocomplete
                  value={field.value}
                  onChange={(_, value) => setValue('identity', value || '')}
                  options={
                    watchNodeType === EnumNodeType.mixnode
                      ? delegations.mixnodes.delegated_nodes
                      : []
                  }
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
      </div>
    </FormControl>
  )
}
