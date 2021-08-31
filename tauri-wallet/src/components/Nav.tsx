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
} from '@material-ui/icons'
import { makeStyles } from '@material-ui/styles'
import { ClientContext } from '../context/main'

const routesSchema = [
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
            <ListItemIcon className={classes.navItem}>{r.Icon}</ListItemIcon>
            <ListItemText
              primary={r.label}
              primaryTypographyProps={{
                className: classes.navItem,
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
