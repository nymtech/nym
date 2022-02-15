import React, { useContext, useState } from 'react'
import { Box, Button, CircularProgress, Grid, LinearProgress, Stack, TextField, Typography } from '@mui/material'
import { PercentOutlined } from '@mui/icons-material'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm } from 'react-hook-form'
import { InfoTooltip } from '../../components/InfoToolTip'
import { InclusionProbabilityResponse, TMixnodeBondDetails } from '../../types'
import { validationSchema } from './validationSchema'
import { updateMixnode } from '../../requests'
import { ClientContext } from '../../context/main'
import { Fee } from '../../components'

type TFormData = {
  profitMarginPercent: number
}

export const SystemVariables = ({
  mixnodeDetails,
  saturation,
  rewardEstimation,
  inclusionProbability,
  onUpdate,
}: {
  mixnodeDetails: TMixnodeBondDetails['mix_node']
  saturation: number
  rewardEstimation: number
  inclusionProbability: InclusionProbabilityResponse
  onUpdate: () => void
}) => {
  const [nodeUpdateResponse, setNodeUpdateResponse] = useState<'success' | 'failed'>()

  const {
    register,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm({
    resolver: yupResolver(validationSchema),
    defaultValues: { profitMarginPercent: mixnodeDetails.profit_margin_percent.toString() },
  })

  const { userBalance, currency } = useContext(ClientContext)

  const onSubmit = async (data: TFormData) => {
    try {
      await updateMixnode({ profitMarginPercent: data.profitMarginPercent })
      await userBalance.fetchBalance()
      onUpdate()
      setNodeUpdateResponse('success')
    } catch (e) {
      setNodeUpdateResponse('failed')
      console.log(e)
    }
  }

  return (
    <>
      <Box sx={{ p: 3 }}>
        <Stack spacing={5}>
          <TextField
            {...register('profitMarginPercent', { valueAsNumber: true })}
            label="Profit margin"
            helperText={
              !!errors.profitMarginPercent
                ? errors.profitMarginPercent.message
                : "The percentage of your delegators' rewards that you as the node operator will take"
            }
            InputProps={{ endAdornment: <PercentOutlined fontSize="small" sx={{ color: 'grey.500' }} /> }}
            error={!!errors.profitMarginPercent}
            disabled={isSubmitting}
          />
          <DataField
            title="Estimated reward"
            info="Estimated reward per epoch for this profit margin if your node is selected in the active set."
            Indicator={
              <Typography sx={{ color: (theme) => theme.palette.nym.fee, fontWeight: '600' }}>
                {rewardEstimation} {currency?.major}
              </Typography>
            }
          />

          <DataField
            title="Estimated chance of being in the active set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<PercentIndicator value={inclusionProbability.in_active} />}
          />
          <DataField
            title="Estimated chance of being in the standby set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<PercentIndicator value={inclusionProbability.in_reserve} />}
          />

          <DataField
            title="Node stake saturation"
            info="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is the total amount of tokens available to stakeholders and K is the number of nodes in the reward set."
            Indicator={<PercentIndicator value={saturation} warning={saturation >= 100} />}
          />
        </Stack>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          p: 3,
          pt: 0,
        }}
      >
        {nodeUpdateResponse === 'success' ? (
          <Typography sx={{ color: 'success.main', fontWeight: 600 }}>Node successfully updated</Typography>
        ) : nodeUpdateResponse === 'failed' ? (
          <Typography sx={{ color: 'error.main', fontWeight: 600 }}>Node update failed</Typography>
        ) : (
          <Fee feeType="UpdateMixnodeConfig" />
        )}
        <Button
          variant="contained"
          color="primary"
          onClick={handleSubmit(onSubmit)}
          disableElevation
          endIcon={isSubmitting && <CircularProgress size={20} />}
          disabled={Object.keys(errors).length > 0 || isSubmitting}
          size="large"
        >
          Update Profit Margin
        </Button>
      </Box>
    </>
  )
}

const DataField = ({ title, info, Indicator }: { title: string; info: string; Indicator: React.ReactElement }) => (
  <Grid container justifyContent="space-between">
    <Grid item xs={12} md={6}>
      <Box display="flex" alignItems="center">
        <InfoTooltip title={info} placement="right" />
        <Typography sx={{ ml: 1 }}>{title}</Typography>
      </Box>
    </Grid>

    <Grid item xs={12} md={6}>
      <Box display="flex" justifyContent="flex-end">
        {Indicator}
      </Box>
    </Grid>
  </Grid>
)

const PercentIndicator = ({ value, warning }: { value: number; warning?: boolean }) => {
  return (
    <Grid container alignItems="center">
      <Grid item xs={2}>
        <Typography component="span" sx={{ color: warning ? 'error.main' : 'nym.fee', fontWeight: 600 }}>
          {value}%
        </Typography>
      </Grid>
      <Grid item xs={10}>
        <LinearProgress
          color="inherit"
          sx={{ color: warning ? 'error.main' : 'nym.fee' }}
          variant="determinate"
          value={value < 100 ? value : 100}
        />
      </Grid>
    </Grid>
  )
}
