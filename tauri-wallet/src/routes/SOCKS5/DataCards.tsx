import React, { useContext } from 'react'
import {
  Box,
  Card,
  CardContent,
  CardHeader,
  CircularProgress,
  Grid,
  Theme,
  Typography,
} from '@material-ui/core'
import { ToggleData } from './Toggle'
import { ArrowDownwardOutlined, ArrowUpwardOutlined } from '@material-ui/icons'
import { makeStyles } from '@material-ui/styles'
import clsx from 'clsx'
import { ClientContext } from '../../context/main'

const useStyles = makeStyles((theme: Theme) => ({
  card: {
    padding: theme.spacing(2),
    height: 250,
  },
  icon: {
    fontSize: 60,
  },
  iconActive: {
    color: theme.palette.primary.main,
  },
  iconInactive: {
    color: theme.palette.grey[800],
  },
}))

export const OutboundCard: React.FC<{ isActive?: boolean }> = ({
  isActive,
}) => {
  const classes = useStyles()
  return (
    <Card className={classes.card} variant="outlined">
      <CardHeader title="Outbound" action={<ToggleData />} />
      <CardContent>
        <Grid container direction="column" alignItems="center">
          <Grid item>
            <ArrowUpwardOutlined
              className={clsx(
                classes.icon,
                isActive ? classes.iconActive : classes.iconInactive
              )}
            />
          </Grid>
          <Grid item>
            {!isActive ? (
              <Typography variant="h3">-</Typography>
            ) : (
              <>
                <Typography variant="h3">
                  298
                  <Typography component="span" color="textSecondary">
                    mb
                  </Typography>
                </Typography>
              </>
            )}
          </Grid>
        </Grid>
      </CardContent>
    </Card>
  )
}

export const InboundCard: React.FC<{ isActive?: boolean }> = ({ isActive }) => {
  const classes = useStyles()
  const { bandwidthUsed } = useContext(ClientContext)
  return (
    <Card className={classes.card} variant="outlined">
      <CardHeader title="Inbound" action={<ToggleData />} />
      <CardContent>
        <Grid container direction="column" alignItems="center">
          <Grid item>
            <ArrowDownwardOutlined
              className={clsx(
                classes.icon,
                isActive ? classes.iconActive : classes.iconInactive
              )}
            />
          </Grid>
          <Grid item>
            {!isActive ? (
              <Typography variant="h3">-</Typography>
            ) : (
              <>
                <Typography variant="h3">
                  {bandwidthUsed}
                  <Typography component="span" color="textSecondary">
                    mb
                  </Typography>
                </Typography>
              </>
            )}
          </Grid>
        </Grid>
      </CardContent>
    </Card>
  )
}

export const LimitCard: React.FC<{ isActive: boolean }> = ({ isActive }) => {
  const classes = useStyles()
  const { bandwidthLimit, bandwidthUsed } = useContext(ClientContext)
  return (
    <Card className={classes.card} variant="outlined">
      <CardHeader title="Usage" action={<ToggleData />} />
      <Grid container direction="column" alignItems="center">
        <Grid item>
          <Box sx={{ position: 'relative', display: 'inline-flex' }}>
            <CircularProgress
              variant="determinate"
              value={100}
              size={120}
              style={{
                color: isActive ? '#eee' : '#aaa',
                transition: 'color 0.5s ease-in-out',
              }}
            />

            <Box>
              <CircularProgress
                variant="determinate"
                value={!isActive ? 0 : (bandwidthUsed / bandwidthLimit) * 100}
                size={120}
                style={{
                  top: 0,
                  left: 0,
                  bottom: 0,
                  right: 0,
                  position: 'absolute',
                }}
              />
            </Box>

            <Box
              sx={{
                top: 0,
                left: 0,
                bottom: 0,
                right: 0,
                position: 'absolute',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
              }}
            >
              {' '}
              {!isActive ? (
                <Typography variant="h3">-</Typography>
              ) : (
                <>
                  <Typography variant="h5">{bandwidthLimit}</Typography>
                  <Typography variant="caption" color="textSecondary">
                    mb
                  </Typography>
                </>
              )}
            </Box>
          </Box>
        </Grid>
      </Grid>
    </Card>
  )
}
