import React, { ChangeEvent, useState } from 'react'
import { Box, Button, Chip, Divider, Grid, LinearProgress, Stack, TextField, Typography } from '@mui/material'
import { AccessTimeOutlined, PercentOutlined } from '@mui/icons-material'
import { InfoTooltip } from '../../components/InfoToolTip'

export const SystemVariables = ({ profitMargin }: { profitMargin: number }) => {
  const [profitMarginPercent, setProfirMarginPercent] = useState<string>(profitMargin.toString())
  return (
    <>
      <Box sx={{ p: 4 }}>
        <Stack spacing={3}>
          <TextField
            label="Profit margin"
            helperText="The percentage of your delegators' rewards that you as the node operator will take"
            value={profitMarginPercent}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setProfirMarginPercent(e.target.value)}
            InputProps={{ endAdornment: <PercentOutlined fontSize="small" sx={{ color: 'grey.500' }} /> }}
          />
          <Divider />
          <DataField
            title="Estimated reward"
            info="Estimated reward per epoch for this profit margin if your node is selected in the active set."
            Indicator={<Typography sx={{ color: 'nym.fee', fontWeight: 600 }}>~ 152,140,028 punk</Typography>}
          />
          <Divider />
          <DataField
            title="Chance of being in the active set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<PercentIndicator value={78} />}
          />
          <DataField
            title="Chance of being in the standby set"
            info="Probability of getting selected in the reward set (active and standby nodes) in the next epoch. The more your stake, the higher the chances to be selected"
            Indicator={<PercentIndicator value={22} />}
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
          justifyContent: 'flex-end',
          borderTop: (theme) => `1px solid ${theme.palette.grey[300]}`,
          bgcolor: 'grey.200',
          padding: 2,
        }}
      >
        <Button variant="contained" color="primary" type="submit" disableElevation>
          Save
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

    <Grid item xs={12} md={5}>
      {Indicator}
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
