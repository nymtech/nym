import { List, ListItem, ListItemIcon, ListItemText } from '@material-ui/core'
import {
  AccountBalanceWalletRounded,
  ArrowBack,
  ArrowForward,
  ExitToApp,
} from '@material-ui/icons'
import React from 'react'
import { Link } from 'react-router-dom'

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
    label: 'Logout',
    route: '/signin',
    Icon: <ExitToApp />,
  },
]

export const Nav = () => (
  <List>
    {routesSchema.map((r, i) => (
      <ListItem button component={Link} to={r.route} key={i} color="red">
        <ListItemIcon>{r.Icon}</ListItemIcon>
        <ListItemText primary={r.label} />
      </ListItem>
    ))}
  </List>
)
