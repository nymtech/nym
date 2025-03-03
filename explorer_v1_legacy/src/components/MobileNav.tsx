import * as React from 'react';
import { useTheme } from '@mui/material/styles';
import { AppBar, Box, Drawer, IconButton, List, ListItem, ListItemButton, ListItemIcon, Toolbar } from '@mui/material';
import { Menu } from '@mui/icons-material';
import { MaintenanceBanner } from '@nymproject/react/banners/MaintenanceBanner';
import { useIsMobile } from '@src/hooks/useIsMobile';
import { useMainContext } from '../context/main';
import { MobileDrawerClose } from '../icons/MobileDrawerClose';
import { Footer } from './Footer';
import { ExpandableButton } from './Nav';
import { ConnectKeplrWallet } from './Wallet/ConnectKeplrWallet';
import NetworkTitle from './NetworkTitle';

export const MobileNav: FCWithChildren = ({ children }) => {
  const theme = useTheme();
  const { navState, updateNavState } = useMainContext();
  const [drawerOpen, setDrawerOpen] = React.useState(false);
  // Set maintenance banner to false by default to don't display it
  const [openMaintenance, setOpenMaintenance] = React.useState(false);
  const isSmallMobile = useIsMobile(400);

  const toggleDrawer = () => {
    setDrawerOpen(!drawerOpen);
  };

  const handleClick = (url: string) => {
    updateNavState(url);
    toggleDrawer();
  };

  const openDrawer = () => {
    setDrawerOpen(true);
  };

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column' }}>
      <AppBar
        sx={{
          background: theme.palette.nym.networkExplorer.topNav.appBar,
          borderRadius: 0,
        }}
      >
        <MaintenanceBanner open={openMaintenance} onClick={() => setOpenMaintenance(false)} />
        <Toolbar
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            alignItems: 'center',
            width: '100%',
          }}
        >
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'row',
              alignItems: 'center',
            }}
          >
            <IconButton onClick={toggleDrawer}>
              <Menu sx={{ color: 'primary.contrastText' }} />
            </IconButton>
            {!isSmallMobile && <NetworkTitle />}
          </Box>
          <ConnectKeplrWallet />
        </Toolbar>
      </AppBar>
      <Drawer
        anchor="left"
        open={drawerOpen}
        onClose={toggleDrawer}
        PaperProps={{
          style: {
            background: theme.palette.nym.networkExplorer.nav.background,
          },
        }}
      >
        <Box role="presentation">
          <List sx={{ pt: 0, pb: 0 }}>
            <ListItem
              disablePadding
              disableGutters
              sx={{
                height: 64,
                background: theme.palette.nym.networkExplorer.nav.background,
                borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
              }}
            >
              <ListItemButton
                onClick={toggleDrawer}
                sx={{
                  pt: 2,
                  pb: 2,
                  background: theme.palette.nym.networkExplorer.nav.background,
                  display: 'flex',
                  justifyContent: 'flex-start',
                }}
              >
                <ListItemIcon>
                  <MobileDrawerClose />
                </ListItemIcon>
              </ListItemButton>
            </ListItem>
            {navState.map((props) => (
              <ExpandableButton
                key={props.url}
                title={props.title}
                openDrawer={openDrawer}
                url={props.url}
                drawIsTempOpen={drawerOpen === true}
                drawIsFixed={false}
                Icon={props.Icon}
                setToActive={handleClick}
                nested={props.nested}
                isMobile
                isActive={props.isActive}
              />
            ))}
          </List>
        </Box>
      </Drawer>

      <Box sx={{ width: '100%', p: 4, mt: 7 }}>
        {children}
        <Footer />
      </Box>
    </Box>
  );
};
