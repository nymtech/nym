import * as React from 'react';
import { Link } from 'react-router-dom';
import { useTheme } from '@mui/material/styles';
import MuiLink from '@mui/material/Link';
import {
  AppBar,
  Box,
  Button,
  Drawer,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  Toolbar,
  Typography,
} from '@mui/material';
import { Menu } from '@mui/icons-material';
import { NymLogo } from '@nymproject/react/logo/NymLogo';
import { useMainContext } from '../context/main';
import { MobileDrawerClose } from '../icons/MobileDrawerClose';
import { Footer } from './Footer';
import { NYM_WEBSITE } from '../api/constants';
import { ExpandableButton } from './Nav';
import { DarkLightSwitchMobile } from './Switch';

type MobileNavProps = {
  children: React.ReactNode;
};

export const MobileNav: React.FC<{ children: React.ReactNode }> = ({ children }: MobileNavProps) => {
  const theme = useTheme();
  const { navState, updateNavState } = useMainContext();
  const [drawerOpen, setDrawerOpen] = React.useState(false);

  const toggleDrawer = () => {
    setDrawerOpen(!drawerOpen);
  };

  const handleClick = (id: number) => {
    updateNavState(id);
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
        }}
      >
        <Toolbar
          disableGutters
          sx={{
            display: 'flex',
            justifyContent: 'space-between',
            width: '100%',
          }}
        >
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'row',
              alignItems: 'center',
              justifyContent: 'space-between',
              width: 'auto',
            }}
          >
            <IconButton component="a" href={NYM_WEBSITE} target="_blank">
              <NymLogo height="40px" width="40px" />
            </IconButton>
            <Typography
              variant="h6"
              noWrap
              sx={{
                color: theme.palette.nym.networkExplorer.nav.text,
                fontSize: '18px',
                fontWeight: 600,
                ml: 2,
              }}
            >
              <MuiLink component={Link} to="/overview" underline="none" color="inherit">
                Network Explorer
              </MuiLink>
            </Typography>
          </Box>

          <Box>
            <DarkLightSwitchMobile />
            <Button onClick={toggleDrawer}>
              <Menu sx={{ color: theme.palette.primary.contrastText }} />
            </Button>
          </Box>
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
                id={props.id}
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
