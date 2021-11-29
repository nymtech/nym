import React, { useContext, useEffect } from 'react'
import { Link, useLocation } from 'react-router-dom'
import { List, ListItem, ListItemIcon, ListItemText } from '@mui/material'
import {
  AccountBalanceWalletOutlined,
  ArrowBack,
  ArrowForward,
  AttachMoney,
  CancelOutlined,
  HowToVoteOutlined,
  MoneyOff,
  Description,
  Settings,
} from '@mui/icons-material'
import { ADMIN_ADDRESS, ClientContext } from '../context/main'

let routesSchema = [
  {
    label: 'Balance',
    route: '/balance',
    Icon: AccountBalanceWalletOutlined,
  },
  {
    label: 'Send',
    route: '/send',
    Icon: ArrowForward,
  },
  {
    label: 'Receive',
    route: '/receive',
    Icon: ArrowBack,
  },
  {
    label: 'Bond',
    route: '/bond',
    Icon: AttachMoney,
  },
  {
    label: 'Unbond',
    route: '/unbond',
    Icon: MoneyOff,
  },
  {
    label: 'Delegate',
    route: '/delegate',
    Icon: HowToVoteOutlined,
  },
  {
    label: 'Undelegate',
    route: '/undelegate',
    Icon: CancelOutlined,
  },
]

export const Nav = () => {
  const { clientDetails, handleShowAdmin } = useContext(ClientContext)
  const location = useLocation()

  useEffect(() => {
    if (clientDetails?.client_address === ADMIN_ADDRESS) {
      routesSchema.push({
        label: 'Docs',
        route: '/docs',
        Icon: Description,
      })
    }
  }, [])

  return (
    <div
      style={{
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'flex-start',
      }}
    >
      <List disablePadding>
        {routesSchema.map(({ Icon, route, label }, i) => (
          <ListItem disableGutters component={Link} to={route} key={i}>
            <ListItemIcon
              sx={{
                minWidth: 30,
                color:
                  location.pathname === route ? 'primary.main' : 'common.white',
              }}
            >
              <Icon sx={{ fontSize: 20 }} />
            </ListItemIcon>
            <ListItemText
              sx={{
                color:
                  location.pathname === route ? 'primary.main' : 'common.white',
              }}
              primary={label}
            />
          </ListItem>
        ))}
        {clientDetails?.client_address === ADMIN_ADDRESS && (
          <ListItem button onClick={handleShowAdmin}>
            <ListItemIcon sx={{ color: 'common.white' }}>
              <Settings />
            </ListItemIcon>
            <ListItemText primary="Admin" sx={{ color: 'common.white' }} />
          </ListItem>
        )}
      </List>
    </div>
  )
}
