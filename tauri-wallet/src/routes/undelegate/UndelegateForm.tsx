import React from 'react'
import { useForm } from 'react-hook-form'
import {
  Button,
  CircularProgress,
  FormControl,
  Grid,
  TextField,
  Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { invoke } from '@tauri-apps/api'
import { yupResolver } from '@hookform/resolvers/yup'
import { validationSchema } from './validationSchema'
import { NodeTypeSelector } from '../../components/NodeTypeSelector'
import { EnumNodeType } from '../../types/global'

type TFormData = {
  nodeType: EnumNodeType
  identity: string
}

const defaultValues = {
  nodeType: EnumNodeType.mixnode,
  identity: '',
}

export const UndelegateForm = ({
  onError,
  onSuccess,
}: {
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

  const onSubmit = async (data: TFormData) => {
    await invoke('undelegate_from_mixnode', { identity: data.identity })
      .then((res: any) => onSuccess(res))
      .catch((e) => onError(e))
  }

  const theme: Theme = useTheme()

  const handleAmountChange = (event: any) => {
    // don't ask me about that. javascript works in mysterious ways
    // and this is apparently a good way of checking if string
    // is purely made of numeric characters
    // const parsed = +event.target.value
    // if (isNaN(parsed)) {
    //   setIsValidAmount(false)
    // } else {
    //   try {
    //     const allocationCheck = { error: undefined, message: '' }
    //     if (allocationCheck.error) {
    //       setAllocationWarning(allocationCheck.message)
    //       setIsValidAmount(false)
    //     } else {
    //       setAllocationWarning(allocationCheck.message)
    //       setIsValidAmount(true)
    //     }
    //   } catch {
    //     setIsValidAmount(false)
    //   }
    // }
  }

  return (
    <FormControl fullWidth>
      <div style={{ padding: theme.spacing(3, 5) }}>
        <Grid container spacing={3} direction="column">
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
