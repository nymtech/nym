import React, { useContext, useEffect, useState } from 'react'
import {
  Box,
  Button,
  Chip,
  CircularProgress,
  Divider,
  Grid,
  LinearProgress,
  Stack,
  TextField,
  Typography,
} from '@mui/material'
import { AccessTimeOutlined, PercentOutlined } from '@mui/icons-material'
import { yupResolver } from '@hookform/resolvers/yup'
import { useForm } from 'react-hook-form'
import { InfoTooltip } from '../../components/InfoToolTip'
import { EnumNodeType, TMixnodeBondDetails } from '../../types'
import { validationSchema } from './validationSchema'
import { bond, unbond } from '../../requests'
import { ClientContext } from '../../context/main'

type TFormData = {
  profitMarginPercent: number
  signature: string
}

export const SystemVariables = ({
  mixnodeDetails,
  pledge,
}: {
  mixnodeDetails: TMixnodeBondDetails['mix_node']
  pledge: TMixnodeBondDetails['pledge_amount']
}) => {
  const [nodeUpdateResponse, setNodeUpdateResponse] = useState<'success' | 'failed'>()

  const {
    register,
    reset,
    handleSubmit,
    formState: { errors, isSubmitting },
  } = useForm({
    resolver: yupResolver(validationSchema),
    defaultValues: { profitMarginPercent: mixnodeDetails.profit_margin_percent.toString() },
  })

  const { userBalance } = useContext(ClientContext)

  useEffect(() => {
    return () => reset()
  }, [])

  const onSubmit = async (data: TFormData) => {
    await unbond(EnumNodeType.mixnode)
    await bond({
      type: EnumNodeType.mixnode,
      data: { ...mixnodeDetails, profit_margin_percent: data.profitMarginPercent },
      pledge: { denom: 'Minor', amount: pledge.amount },
      //hardcoded for the moment as required in bonding but not necessary
      ownerSignature: 'foo',
    })
      .then(() => {
        userBalance.fetchBalance()
        setNodeUpdateResponse('success')
      })
      .catch((e) => {
        setNodeUpdateResponse('failed')
        console.log(e)
      })
  }

  return (
    <>
      <Box sx={{ p: 4 }}>
        <Stack spacing={3}>
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
          <Divider />
          <DataField
            title="Estimated reward"
            info="Estimated reward per epoch for this profit margin if your node is selected in the active set."
            Indicator={<Chip label="Coming soon" icon={<AccessTimeOutlined fontSize="small" />} />}
          />
          <Divider />
          <DataField
            title="Chance of being in the active set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<Chip label="Coming soon" icon={<AccessTimeOutlined fontSize="small" />} />}
          />
          <DataField
            title="Chance of being in the standby set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<Chip label="Coming soon" icon={<AccessTimeOutlined fontSize="small" />} />}
          />

          <Divider />
          <DataField
            title="Node stake saturation"
            info="Level of stake saturation for this node. Nodes receive more rewards the higher their saturation level, up to 100%. Beyond 100% no additional rewards are granted. The current stake saturation level is: 1 million NYM, computed as S/K where S is the total amount of tokens available to stakeholders and K is the number of nodes in the reward set."
            Indicator={<Chip label="Coming soon" icon={<AccessTimeOutlined fontSize="small" />} />}
          />
        </Stack>
      </Box>
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderTop: (theme) => `1px solid ${theme.palette.grey[300]}`,
          bgcolor: 'grey.200',
          padding: 2,
        }}
      >
        {nodeUpdateResponse === 'success' ? (
          <Typography sx={{ color: 'success.main', fontWeight: 600 }}>Node successfully updated</Typography>
        ) : nodeUpdateResponse === 'failed' ? (
          <Typography sx={{ color: 'error.main', fontWeight: 600 }}>Node updated failed</Typography>
        ) : (
          <Box />
        )}
        <Button
          variant="contained"
          color="primary"
          onClick={handleSubmit(onSubmit)}
          disableElevation
          endIcon={isSubmitting && <CircularProgress size={20} />}
          disabled={Object.keys(errors).length > 0 || isSubmitting}
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

const PercentIndicator = ({ value }: { value: number }) => {
  return (
    <Grid container alignItems="center">
      <Grid item xs={2}>
        <Typography component="span" sx={{ color: 'nym.fee', fontWeight: 600 }}>
          {value}%
        </Typography>
      </Grid>
      <Grid item xs={10}>
        <LinearProgress color="inherit" sx={{ color: 'nym.fee' }} variant="determinate" value={value} />
      </Grid>
    </Grid>
  )
}
