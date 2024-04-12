'use client'

import * as React from 'react'
import { ExpandLess, ExpandMore, Menu } from '@mui/icons-material'
import { CSSObject, styled, Theme, useTheme } from '@mui/material/styles'
import { Link as MuiLink } from '@mui/material'
import Button from '@mui/material/Button'
import Box from '@mui/material/Box'
import ListItem from '@mui/material/ListItem'
import MuiDrawer from '@mui/material/Drawer'
import AppBar from '@mui/material/AppBar'
import Toolbar from '@mui/material/Toolbar'
import Typography from '@mui/material/Typography'
import List from '@mui/material/List'
import IconButton from '@mui/material/IconButton'
import ListItemButton from '@mui/material/ListItemButton'
import ListItemIcon from '@mui/material/ListItemIcon'
import ListItemText from '@mui/material/ListItemText'
import { NYM_WEBSITE } from '@/app/api/constants'
import { useMainContext } from '@/app/context/main'
import { MobileDrawerClose } from '@/app/icons/MobileDrawerClose'
import { NavOptionType, originalNavOptions } from '@/app/context/nav'
import { DarkLightSwitchDesktop } from '@/app/components/Switch'
import { Footer } from '@/app/components/Footer'
import { ConnectKeplrWallet } from '@/app/components/Wallet/ConnectKeplrWallet'
import { usePathname, useRouter } from 'next/navigation'

const drawerWidth = 255
const bannerHeight = 80

const openedMixin = (theme: Theme): CSSObject => ({
  width: drawerWidth,
  transition: theme.transitions.create('width', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.enteringScreen,
  }),
  overflowX: 'hidden',
})

const closedMixin = (theme: Theme): CSSObject => ({
  transition: theme.transitions.create('width', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  }),
  overflowX: 'hidden',
  width: `calc(${theme.spacing(7)} + 1px)`,
})

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  height: 64,
}))

const Drawer = styled(MuiDrawer, {
  shouldForwardProp: (prop) => prop !== 'open',
})(({ theme, open }) => ({
  width: drawerWidth,
  flexShrink: 0,
  whiteSpace: 'nowrap',
  boxSizing: 'border-box',
  ...(open && {
    ...openedMixin(theme),
    '& .MuiDrawer-paper': openedMixin(theme),
  }),
  ...(!open && {
    ...closedMixin(theme),
    '& .MuiDrawer-paper': closedMixin(theme),
  }),
}))

type ExpandableButtonType = {
  title: string
  url: string
  isActive?: boolean
  Icon?: React.ReactNode
  nested?: NavOptionType[]
  isChild?: boolean
  isMobile: boolean
  drawIsTempOpen: boolean
  drawIsFixed: boolean
  isExternalLink?: boolean
  openDrawer: () => void
  closeDrawer?: () => void
  fixDrawerClose?: () => void
}

export const ExpandableButton: FCWithChildren<ExpandableButtonType> = ({
  title,
  url,
  drawIsTempOpen,
  drawIsFixed,
  Icon,
  nested,
  isMobile,
  isChild,
  isExternalLink,
  openDrawer,
  closeDrawer,
  fixDrawerClose,
}) => {
  const { palette } = useTheme()
  const pathname = usePathname()
  const router = useRouter()

  const handleClick = () => {
    if (title === 'Network Components') {
      return undefined
    }

    if (isExternalLink) {
      window.open(url, '_blank')

      return undefined
    }

    if (!isExternalLink) {
      router.push(url, {})
    }

    if (closeDrawer) {
      closeDrawer()
    }
  }
  const selectedStyle = {
    background: palette.nym.networkExplorer.nav.selected.main,
    borderRight: `3px solid ${palette.nym.highlight}`,
  }

  return (
    <>
      <ListItem
        disablePadding
        disableGutters
        sx={{
          borderBottom: isChild ? 'none' : '1px solid rgba(255, 255, 255, 0.1)',
          ...(pathname === url
            ? selectedStyle
            : {
                background: palette.nym.networkExplorer.nav.background,
                borderRight: 'none',
              }),
        }}
      >
        <ListItemButton
          onClick={() => handleClick()}
          sx={{
            pt: 2,
            pb: 2,
            background: isChild
              ? palette.nym.networkExplorer.nav.selected.nested
              : 'none',
          }}
        >
          <ListItemIcon sx={{ minWidth: '39px' }}>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: palette.nym.networkExplorer.nav.text,
            }}
          />
        </ListItemButton>
      </ListItem>
      {nested?.map((each) => (
        <ExpandableButton
          url={each.url}
          key={each.title}
          title={each.title}
          openDrawer={openDrawer}
          drawIsTempOpen={drawIsTempOpen}
          closeDrawer={closeDrawer}
          drawIsFixed={drawIsFixed}
          fixDrawerClose={fixDrawerClose}
          isMobile={isMobile}
          isChild
          isExternalLink={each.isExternal}
        />
      ))}
    </>
  )
}

export const Nav: FCWithChildren = ({ children }) => {
  const { environment } = useMainContext()
  const [drawerIsOpen, setDrawerToOpen] = React.useState(false)
  const [fixedOpen, setFixedOpen] = React.useState(false)
  // Set maintenance banner to false by default to don't display it
  const [openMaintenance, setOpenMaintenance] = React.useState(false)
  const theme = useTheme()

  const explorerName = environment
    ? `${environment} Explorer`
    : 'Mainnet Explorer'

  const switchNetworkText =
    environment === 'mainnet' ? 'Switch to Testnet' : 'Switch to Mainnet'
  const switchNetworkLink =
    environment === 'mainnet'
      ? 'https://sandbox-explorer.nymtech.net'
      : 'https://explorer.nymtech.net'

  const fixDrawerOpen = () => {
    setFixedOpen(true)
    setDrawerToOpen(true)
  }

  const fixDrawerClose = () => {
    setFixedOpen(false)
    setDrawerToOpen(false)
  }

  const tempDrawerOpen = () => {
    if (!fixedOpen) {
      setDrawerToOpen(true)
    }
  }

  const tempDrawerClose = () => {
    if (!fixedOpen) {
      setDrawerToOpen(false)
    }
  }

  return (
    <Box sx={{ display: 'flex' }}>
      <AppBar
        sx={{
          background: theme.palette.nym.networkExplorer.topNav.appBar,
          borderRadius: 0,
        }}
      >
        <Toolbar
          disableGutters
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
          }}
        >
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'row',
              alignItems: 'center',
              justifyContent: 'space-between',
              ml: 0.5,
            }}
          >
            <IconButton component="a" href={NYM_WEBSITE} target="_blank">
              {/* <NymLogo /> */}
            </IconButton>
            <Typography
              variant="h6"
              noWrap
              sx={{
                color: theme.palette.nym.networkExplorer.nav.text,
                fontSize: '18px',
                fontWeight: 600,
              }}
            >
              <MuiLink
                href="/"
                underline="none"
                color="inherit"
                textTransform="capitalize"
              >
                {explorerName}
              </MuiLink>
              <Button
                size="small"
                variant="outlined"
                color="inherit"
                href={switchNetworkLink}
                sx={{
                  borderRadius: 2,
                  textTransform: 'none',
                  width: 150,
                  ml: 4,
                  fontSize: 14,
                  fontWeight: 600,
                }}
              >
                {switchNetworkText}
              </Button>
            </Typography>
          </Box>
          <Box
            sx={{
              mr: 2,
              alignItems: 'center',
              display: 'flex',
            }}
          >
            <Box
              sx={{
                display: 'flex',
                flexDirection: 'row',
                width: 'auto',
                pr: 0,
                pl: 2,
                justifyContent: 'flex-end',
                alignItems: 'center',
              }}
            >
              <Box sx={{ mr: 1 }}>
                <ConnectKeplrWallet />
              </Box>
              <DarkLightSwitchDesktop defaultChecked />
            </Box>
          </Box>
        </Toolbar>
      </AppBar>
      <Drawer
        variant="permanent"
        open={true}
        PaperProps={{
          style: {
            background: theme.palette.nym.networkExplorer.nav.background,
            borderRadius: 0,
            top: openMaintenance ? bannerHeight : 0,
          },
        }}
      >
        <DrawerHeader
          sx={{
            borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
            justifyContent: 'flex-start',
            paddingLeft: 0,
            display: 'none',
          }}
        >
          <IconButton
            onClick={drawerIsOpen ? fixDrawerClose : fixDrawerOpen}
            sx={{
              padding: 1,
              ml: 1,
              color: theme.palette.nym.networkExplorer.nav.text,
            }}
          >
            {drawerIsOpen ? <MobileDrawerClose /> : <Menu />}
          </IconButton>
        </DrawerHeader>

        <List
          sx={{ pb: 0 }}
          onMouseEnter={tempDrawerOpen}
          onMouseLeave={tempDrawerClose}
        >
          {originalNavOptions.map((props) => (
            <ExpandableButton
              key={props.url}
              closeDrawer={tempDrawerClose}
              drawIsTempOpen={drawerIsOpen}
              drawIsFixed={fixedOpen}
              fixDrawerClose={fixDrawerClose}
              openDrawer={tempDrawerOpen}
              isMobile={false}
              {...props}
            />
          ))}
        </List>
      </Drawer>
      <Box
        style={{ width: `calc(100% - ${drawerWidth}px` }}
        sx={{ py: 5, px: 6, mt: 7 }}
      >
        {children}
        <Footer />
      </Box>
    </Box>
  )
}
