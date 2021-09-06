import React from 'react'
import {
  Button,
  CircularProgress,
  FormControl,
  Grid,
  InputAdornment,
  TextField,
  Theme,
  useTheme,
} from '@material-ui/core'
import { useForm } from 'react-hook-form'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { EnumNodeType } from '../../types/global'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { invoke } from '@tauri-apps/api'

type TDelegateForm = {
  nodeType: EnumNodeType
  identity: string
  amount: string
}

const defaultValues: TDelegateForm = {
  nodeType: EnumNodeType.Mixnode,
  identity: '',
  amount: '',
}

export const DelegateForm = ({
  onError,
  onSuccess,
}: {
  onError: (message?: string) => void
  onSuccess: (message?: string) => void
}) => {
  const theme = useTheme<Theme>()
  const {
    register,
    setValue,
    watch,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm<TDelegateForm>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  })

  const watchNodeType = watch('nodeType', defaultValues.nodeType)

  const onSubmit = async (data: TDelegateForm) => {
    await invoke('delegate_to_mixnode', {
      identity: data.identity,
      amount: { denom: 'punk', amount: data.amount },
    })
      .then((res: any) => {
        console.log(res)
        onSuccess(res)
      })
      .catch((e) => {
        console.log(e)
        onError(e)
      })
  }

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3}>
          <Grid item xs={12}>
            <NodeTypeSelector
              nodeType={watchNodeType}
              setNodeType={(nodeType) => setValue('nodeType', nodeType)}
            />
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('identity')}
              required
              variant="outlined"
              id="identity"
              name="identity"
              label="Node identity"
              fullWidth
              error={!!errors.identity}
              helperText={errors?.identity?.message}
            />
          </Grid>

          <Grid item xs={12} lg={6}>
            <TextField
              {...register('amount')}
              required
              variant="outlined"
              id="amount"
              name="amount"
              label="Amount to delegate"
              fullWidth
              error={!!errors.amount}
              helperText={errors?.amount?.message}
              InputProps={{
                endAdornment: (
                  <InputAdornment position="end">punks</InputAdornment>
                ),
              }}
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
          disabled={isSubmitting}
          variant="contained"
          color="primary"
          type="submit"
          disableElevation
          endIcon={isSubmitting && <CircularProgress size={20} />}
        >
          Delegate stake
        </Button>
      </div>
    </FormControl>
  )
}
