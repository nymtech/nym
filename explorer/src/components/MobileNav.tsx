import * as React from 'react';
import { Link } from 'react-router-dom';
import { CSSObject, Theme, useTheme, styled } from '@mui/material/styles';
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
  ListItemText,
  Toolbar,
  Typography,
} from '@mui/material';
import { ExpandMore, Menu } from '@mui/icons-material';
import { NymLogoSVG } from 'src/icons/NymLogoSVG';
import { useMainContext } from 'src/context/main';
import { Footer } from './Footer';
import { NYM_WEBSITE } from '../api/constants';
import { ExpandableButton } from './Nav';

type MobileNavProps = {
  children: React.ReactNode;
};

const background = '#242C3D';

export const MobileNav: React.FC<{ children: React.ReactNode }> = ({
  children,
}: MobileNavProps) => {
  const theme = useTheme();
  const { navState, updateNavState } = useMainContext();
  const [nestedOptions, toggleNestedOptions] = React.useState(false);
  const [drawerOpen, setDrawerOpen] = React.useState(true);

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

  console.log('nav state is now', navState);

  return (
    <Box sx={{ display: 'flex', flexDirection: 'column' }}>
      <AppBar
        sx={{
          background: theme.palette.nym.networkExplorer.nav.background,
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
              <NymLogoSVG />
            </IconButton>
            <Typography
              variant="h6"
              noWrap
              sx={{
                color: theme.palette.nym.networkExplorer.nav.text,
                fontSize: '18px',
                fontWeight: 800,
                ml: 2,
              }}
            >
              <MuiLink
                component={Link}
                to="/overview"
                underline="none"
                color="inherit"
              >
                Network Explorer
              </MuiLink>
            </Typography>
          </Box>
          <Button onClick={toggleDrawer}>
            <Menu sx={{ color: theme.palette.primary.contrastText }} />
          </Button>
        </Toolbar>
      </AppBar>
      <Drawer anchor="left" open={drawerOpen} onClose={toggleDrawer}>
        <Box role="presentation">
          <List sx={{ pt: 0, pb: 0, background: '#111826' }}>
            <ListItem disablePadding disableGutters>
              <ListItemButton
                onClick={toggleDrawer}
                sx={{
                  pt: 2,
                  pb: 2,
                  background,
                }}
              >
                <ListItemIcon>🆇</ListItemIcon>
              </ListItemButton>
            </ListItem>
          </List>
          <List sx={{ pt: 0, pb: 0 }}>
            {navState.map((props) => (
              <ExpandableButton
                key={props.url}
                id={props.id}
                title={props.title}
                openDrawer={openDrawer}
                url={props.url}
                closeDrawer={() => null}
                drawIsTempOpen={drawerOpen === true}
                drawIsFixed={false}
                fixDrawerClose={() => null}
                Icon={props.Icon}
                setToActive={handleClick}
                nested={props.nested}
                isMobile
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
