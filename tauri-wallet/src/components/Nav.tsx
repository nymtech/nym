import React, { useContext } from 'react'
import { Link, useLocation } from 'react-router-dom'
import {
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Theme,
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
} from '@material-ui/icons'
import { makeStyles } from '@material-ui/styles'
import clsx from 'clsx'
import { ClientContext } from '../context/main'

let routesSchema = [
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
]

if (process.env.NODE_ENV) {
  routesSchema.push({
    label: 'Docs',
    route: '/docs',
    Icon: <Description />,
  })
}

const useStyles = makeStyles((theme: Theme) => ({
  navItem: {
    color: '#fff',
    fontSize: 24,
  },
  selected: {
    color: theme.palette.primary.main,
  },
}))

export const Nav = () => {
  const classes = useStyles()
  const { logOut } = useContext(ClientContext)
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
        {routesSchema.map((r, i) => (
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
