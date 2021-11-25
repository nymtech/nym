import React, { useContext, useEffect } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material'
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
} from '@mui/icons-material'

import { ADMIN_ADDRESS, ClientContext } from '../context/main'

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

export const Nav = () => {
  const { clientDetails, handleShowAdmin, logOut } = useContext(ClientContext)
  const location = useLocation()

  useEffect(() => {
    if (clientDetails?.client_address === ADMIN_ADDRESS) {
      routesSchema.push({
        label: 'Docs',
        route: '/docs',
        Icon: <Description />,
      })
    }
  }, [])

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
            <ListItemIcon>{r.Icon}</ListItemIcon>
            <ListItemText primary={r.label} />
          </ListItem>
        ))}
        {clientDetails?.client_address === ADMIN_ADDRESS && (
          <ListItem button onClick={handleShowAdmin}>
            <ListItemIcon>
              <Settings />
            </ListItemIcon>
            <ListItemText primary="Admin" />
          </ListItem>
        )}

        <ListItem button onClick={logOut}>
          <ListItemIcon data-testid="log-out">
            <ExitToApp />
          </ListItemIcon>
          <ListItemText primary="Log out" />
        </ListItem>
      </List>
    </div>
  )
}
