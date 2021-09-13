import React, { useContext } from 'react'
import { useForm } from 'react-hook-form'
import {
  Button,
  CircularProgress,
  FormControl,
  Grid,
  TextField,
  Theme,
} from '@material-ui/core'
import { Alert } from '@material-ui/lab'
import { useTheme } from '@material-ui/styles'
import { invoke } from '@tauri-apps/api'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { EnumNodeType, TFee } from '../../types'
import { ClientContext } from '../../context/main'

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
  onError,
  onSuccess,
}: {
  fees: TFee
  onError: (message?: string) => void
  onSuccess: (message?: string) => void
}) => {
  const {
    handleSubmit,
    register,
    setValue,
    watch,
    formState: { errors, isSubmitting },
  } = useForm<TFormData>({
    defaultValues,
    resolver: yupResolver(validationSchema),
  })
  const watchNodeType = watch('nodeType')
  const { getBalance } = useContext(ClientContext)

  const onSubmit = async (data: TFormData) => {
    await invoke(`undelegate_from_${data.nodeType}`, {
      identity: data.identity,
    })
      .then((res: any) => {
        onSuccess(res)
        getBalance.fetchBalance()
      })
      .catch((e) => onError(e))
  }

  const theme: Theme = useTheme()

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3} direction="column">
          <Grid container item xs={12} justifyContent="space-between">
            <Grid item>
              <NodeTypeSelector
                nodeType={watchNodeType}
                setNodeType={(nodeType) => setValue('nodeType', nodeType)}
              />
            </Grid>
            <Grid item>
              <Alert severity="info">
                {`A fee of ${
                  watchNodeType === EnumNodeType.mixnode
                    ? fees.mixnode.amount
                    : fees.gateway.amount
                } PUNK will apply to this transaction`}
              </Alert>
            </Grid>
          </Grid>
          <Grid item xs={12}>
            <TextField
              {...register('identity')}
              required
              variant="outlined"
              id="identity"
              name="identity"
              label="Node identity"
              error={!!errors.identity}
              helperText={errors.identity?.message}
              fullWidth
            />
          </Grid>

          {/* {allocationWarning && (
            <Grid item xs={12} lg={6}>
              <Alert severity={!isValidAmount ? 'error' : 'info'}>
                {allocationWarning}
              </Alert>
            </Grid>
          )} */}
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
