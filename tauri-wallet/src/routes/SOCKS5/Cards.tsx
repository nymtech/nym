import React from 'react'
import {
  Box,
  Button,
  Card,
  CardContent,
  CardHeader,
  Chip,
  IconButton,
  Theme,
  Typography,
} from '@material-ui/core'
import {
  Cancel,
  CheckCircle,
  PowerSettingsNew,
  PowerSettingsNewSharp,
  SecuritySharp,
} from '@material-ui/icons'
import { useTheme } from '@material-ui/styles'

const ActiveChip = () => {
  const theme: Theme = useTheme()
  return (
    <Chip
      label="Secure"
      style={{
        color: theme.palette.common.white,
        backgroundColor: theme.palette.success.main,
      }}
      icon={<CheckCircle style={{ color: theme.palette.common.white }} />}
    />
  )
}

const InactiveChip = () => {
  const theme: Theme = useTheme()
  return (
    <Chip
      label="Offline"
      style={{
        color: theme.palette.common.white,
        backgroundColor: theme.palette.error.main,
      }}
      icon={<Cancel style={{ color: theme.palette.common.white }} />}
    />
  )
}

export const TopCard: React.FC<{
  isActive: boolean
  disabled: boolean
  plan: string
  toggleIsActive: () => void
}> = ({ isActive, disabled, plan, toggleIsActive }) => {
  const theme: Theme = useTheme()
  return (
    <Card style={{ padding: theme.spacing(1.5) }} variant="outlined">
      <CardHeader
        title={<Typography variant="h5">Package: {plan}</Typography>}
        avatar={isActive ? <ActiveChip /> : <InactiveChip />}
        action={
          <IconButton
            onClick={toggleIsActive}
            disabled={disabled}
            style={
              !disabled
                ? {
                    color: isActive
                      ? theme.palette.success.main
                      : theme.palette.error.main,
                  }
                : {}
            }
          >
            <PowerSettingsNew />
          </IconButton>
        }
      />
    </Card>
  )
}

export const MainCard: React.FC<{
  isActive: boolean
  disabled?: boolean
  buyBandwidth: () => void
  toggleIsActive: () => void
}> = ({ isActive, disabled, buyBandwidth, toggleIsActive }) => {
  const theme: Theme = useTheme()

  return (
    <div style={{ position: 'relative', width: '100%' }}>
      <Card variant={'outlined'} style={{ padding: theme.spacing(2) }}>
        <CardHeader
          title={<Typography> SOCKS5</Typography>}
          subheader={
            isActive
              ? "You're protected with SOCKS5"
              : 'SOCKS5 is not currently active'
          }
        />
        <CardContent>
          <Box display="flex" justifyContent="flex-end">
            <Button
              color="primary"
              variant="contained"
              endIcon={<SecuritySharp />}
              style={{
                color: theme.palette.common.white,
                marginRight: theme.spacing(1.5),
              }}
              size="large"
              disableElevation
              onClick={buyBandwidth}
            >
              Puchase bandwidth
            </Button>
            <Button
              variant="outlined"
              color="primary"
              endIcon={<PowerSettingsNewSharp />}
              size="large"
              disableElevation
              onClick={toggleIsActive}
              disabled={disabled}
            >
              {isActive ? 'Disabled' : 'Enable'}
            </Button>
          </Box>
        </CardContent>
      </Card>
    </div>
  )
}
