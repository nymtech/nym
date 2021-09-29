import React, { useState } from 'react'
import {
  Box,
  Button,
  Card,
  CardActions,
  CardContent,
  CardHeader,
  Chip,
  Grid,
  TextField,
  Theme,
  Typography,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { SecuritySharp } from '@material-ui/icons'
import { Autocomplete } from '@material-ui/lab'
import { NymCard } from '../../components'
import { Info } from './Info'

type TSetupProps = {
  handleSelectPlan: (plan: string) => void
}

export const Setup: React.FC<TSetupProps> = ({ handleSelectPlan }) => {
  const theme: Theme = useTheme()

  const [userSelection, setUserSelection] = useState<string | null>(null)

  return (
    <NymCard
      title="SOCKS5 - Purchase bandwidth"
      subheader="Purchase badwidth to get started with SOCKS5"
      Action={<Info />}
    >
      <Box padding={theme.spacing(0.5)}>
        <Grid container direction="column" alignItems="center" spacing={5}>
          <Grid item container spacing={3} justifyContent="space-evenly">
            <Grid item xs={12} lg={3}>
              <OptionCard
                title="500MB"
                cost="500 PUNK"
                onSelect={handleSelectPlan}
              />
            </Grid>
            <Grid item xs={12} lg={3}>
              <OptionCard
                title="1GB"
                cost="750 PUNK"
                onSelect={handleSelectPlan}
              />
            </Grid>
            <Grid item xs={12} lg={3}>
              <OptionCard
                title="10GB"
                cost="7000 PUNK"
                onSelect={handleSelectPlan}
              />
            </Grid>
          </Grid>

          <Grid item>
            <Typography variant="h5">- OR -</Typography>
          </Grid>

          <Grid
            container
            item
            justifyContent="center"
            alignItems="center"
            style={{ margin: 'auto' }}
          >
            <Grid item>
              <Autocomplete
                disablePortal
                onChange={(_, val: string | null) => setUserSelection(val)}
                style={{ width: 500 }}
                id="bandwidth-options"
                options={[
                  '1MB',
                  '25MB',
                  '50MB',
                  '100MB',
                  '500MB',
                  '1GB',
                  '5GB',
                  '10GB',
                  '20GB',
                  '50GB',
                ]}
                renderInput={(params) => (
                  <TextField
                    {...params}
                    variant="outlined"
                    label="Other options"
                  />
                )}
              />
            </Grid>
            <Grid item>
              <Button
                onClick={() =>
                  userSelection ? handleSelectPlan(userSelection) : undefined
                }
                variant="outlined"
                color="primary"
                size="small"
                disabled={!userSelection}
                style={{ marginLeft: theme.spacing(1) }}
              >
                Select
              </Button>
            </Grid>
          </Grid>
        </Grid>
      </Box>
    </NymCard>
  )
}

type TOptionProps = {
  title: string
  cost: string
  isPrimary?: boolean
  onSelect: TSetupProps['handleSelectPlan']
}
const OptionCard: React.FC<TOptionProps> = ({
  title,
  cost,
  isPrimary,
  onSelect,
}) => {
  const theme: Theme = useTheme()
  return (
    <Card
      variant="outlined"
      style={{ padding: theme.spacing(2), position: 'relative', width: 300 }}
    >
      <CardHeader
        title={
          <Box display="flex" alignItems="end" justifyContent="flex-start">
            <SecuritySharp
              style={{ marginRight: theme.spacing(0.5) }}
              color="action"
            />
            <Typography variant="h5">{title}</Typography>
          </Box>
        }
        color="primary"
        action={<Chip label={cost} />}
      />

      <CardActions>
        <Box display="flex" justifyContent="center" width="100%">
          <Button
            color="primary"
            variant={isPrimary ? 'contained' : 'outlined'}
            disableElevation
            size="small"
            onClick={() => onSelect(title)}
          >
            Select
          </Button>
        </Box>
      </CardActions>
    </Card>
  )
}
