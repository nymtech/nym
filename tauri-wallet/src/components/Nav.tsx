import React, { useContext } from 'react'
import { Link, useLocation } from 'react-router-dom'
import {
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Theme,
  Typography,
} from '@material-ui/core'
import {
  AccountBalanceWalletRounded,
  ArrowBack,
  ArrowForward,
  AttachMoney,
  Cancel,
  ExitToApp,
  HowToVote,
  MoneyOff,
  Description,
  Settings,
  VpnLockSharp,
} from '@material-ui/icons'
import { makeStyles, useTheme } from '@material-ui/styles'
import clsx from 'clsx'
import { ADMIN_ADDRESS, ClientContext } from '../context/main'

const RoutesSchema = () => {
  const routes = [
    {
      label: 'Balance',
      route: '/balance',
      Icon: <AccountBalanceWalletRounded />,
    },
    {
      label: 'Send',
      route: '/send',
      Icon: <ArrowForward />,
    },
    {
      label: 'Receive',
      route: '/receive',
      Icon: <ArrowBack />,
    },
    {
      label: 'Bond',
      route: '/bond',
      Icon: <AttachMoney />,
    },
    {
      label: 'Unbond',
      route: '/unbond',
      Icon: <MoneyOff />,
    },
    {
      label: 'Delegate',
      route: '/delegate',
      Icon: <HowToVote />,
    },
    {
      label: 'Undelegate',
      route: '/undelegate',
      Icon: <Cancel />,
    },
    {
      label: 'SOCKS5',
      route: '/socks5',
      Icon: <VpnLockSharp />,
    },
  ]

  if (process.env.NODE_ENV) {
    routes.push({
      label: 'Docs',
      route: '/docs',
      Icon: <Description />,
    })
  }

  return routes
}

const useStyles = makeStyles((theme: Theme) => ({
  navItem: {
    color: theme.palette.common.white,
    fontSize: 24,
  },
  selected: {
    color: theme.palette.primary.main,
  },
}))

export const Nav = () => {
  const classes = useStyles()
  const { clientDetails, handleShowAdmin, logOut } = useContext(ClientContext)
  const location = useLocation()

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <List>
        {RoutesSchema().map((r, i) => (
          <ListItem button component={Link} to={r.route} key={i}>
            <ListItemIcon
              className={clsx([
                classes.navItem,
                location.pathname === r.route ? classes.selected : undefined,
              ])}
            >
              {r.Icon}
            </ListItemIcon>
            <ListItemText
              primary={r.label}
              primaryTypographyProps={{
                className: clsx([
                  classes.navItem,
                  location.pathname === r.route ? classes.selected : undefined,
                ]),
              }}
            />
          </ListItem>
        ))}
        {clientDetails?.client_address === ADMIN_ADDRESS && (
          <ListItem button onClick={handleShowAdmin}>
            <ListItemIcon className={classes.navItem}>
              <Settings />
            </ListItemIcon>
            <ListItemText
              primary="Admin"
              primaryTypographyProps={{
                className: classes.navItem,
              }}
            />
          </ListItem>
        )}

        <ListItem button onClick={logOut}>
          <ListItemIcon className={classes.navItem}>
            <ExitToApp />
          </ListItemIcon>
          <ListItemText
            primary="Log out"
            primaryTypographyProps={{
              className: classes.navItem,
            }}
          />
        </ListItem>
      </List>
    </div>
  )
}
