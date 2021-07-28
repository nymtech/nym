import {
  AppBar,
  Divider,
  Drawer,
  IconButton,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListSubheader,
  Toolbar,
  Typography,
} from '@material-ui/core'
import React, { useContext } from 'react'
import Link from 'next/link'
import VpnKeyIcon from '@material-ui/icons/VpnKey'
import AccountBalanceWalletIcon from '@material-ui/icons/AccountBalanceWallet'
import { ValidatorClientContext } from '../contexts/ValidatorClient'
import { ADMIN_ADDRESS } from '../pages/_app'
import MenuIcon from '@material-ui/icons/Menu'
import {
  MonetizationOn,
  AttachMoney,
  MoneyOff,
  HowToVote,
  Cancel,
  Pageview,
  ArrowRightAlt,
  ArrowLeft,
  ArrowRightSharp,
  ArrowForward,
  ArrowBack,
} from '@material-ui/icons'

import { makeBasicStyle } from '../common/helpers'
import { theme } from '../lib/theme'

export default function MainNav() {
  const classes = makeBasicStyle(theme)

  const { client } = useContext(ValidatorClientContext)

  let adminPageDisplayed = false

  if (client !== null && client.address === ADMIN_ADDRESS) {
    adminPageDisplayed = true
  }

  const [open, setOpen] = React.useState(false)

  const toggleDrawer = () => {
    setOpen((prevOpen) => !prevOpen)
  }

  const closeDrawer = () => {
    setOpen(false)
  }

  return (
    <>
      <AppBar position="absolute" color="default" className={classes.appBar}>
        <Toolbar>
          <IconButton
            edge="start"
            className={classes.menuButton}
            color="inherit"
            aria-label="menu"
            onClick={toggleDrawer}
          >
            <MenuIcon />
          </IconButton>

          <Drawer anchor={'left'} open={open} onClose={closeDrawer}>
            <div
              className={classes.list}
              role="presentation"
              onClick={closeDrawer}
            >
              <List
                component="nav"
                aria-labelledby="list-header"
                subheader={
                  <ListSubheader id="list-header">Nym Wallet</ListSubheader>
                }
              >
                <Divider />

                <ListItem button>
                  <ListItemIcon>
                    <AccountBalanceWalletIcon />
                  </ListItemIcon>
                  <Link href="/balanceCheck">
                    <ListItemText primary="Check Balance" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <ArrowForward />
                  </ListItemIcon>
                  <Link href="/send">
                    <ListItemText primary="Send coins" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <ArrowBack />
                  </ListItemIcon>
                  <Link href="/receive">
                    <ListItemText primary="Receive coins" />
                  </Link>
                </ListItem>

                <Divider />

                {/*<ListItem>*/}
                {/*    <ListItemText primary="Node management" secondary="bottom text"/>*/}
                {/*</ListItem>*/}

                <ListItem button>
                  <ListItemIcon>
                    <AttachMoney />
                  </ListItemIcon>
                  <Link href="/bond">
                    <ListItemText primary="Bond node" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <MoneyOff />
                  </ListItemIcon>
                  <Link href="/unbond">
                    <ListItemText primary="Unbond node" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <HowToVote />
                  </ListItemIcon>
                  <Link href="/delegateStake">
                    <ListItemText primary="Delegate stake" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <Cancel />
                  </ListItemIcon>
                  <Link href="/undelegateStake">
                    <ListItemText primary="Undelegate stake" />
                  </Link>
                </ListItem>

                <ListItem button>
                  <ListItemIcon>
                    <Pageview />
                  </ListItemIcon>
                  <Link href="/checkDelegation">
                    <ListItemText primary="Check current delegation" />
                  </Link>
                </ListItem>

                {adminPageDisplayed && (
                  <>
                    <Divider />
                    <ListItem button>
                      <ListItemIcon>
                        <VpnKeyIcon />
                      </ListItemIcon>

                      <Link href="/admin">
                        <ListItemText primary="Admin" />
                      </Link>
                    </ListItem>
                  </>
                )}
              </List>
            </div>
          </Drawer>

          <Typography variant="h6" color="inherit" noWrap>
            Nym
          </Typography>
        </Toolbar>
      </AppBar>
    </>
  )
}
