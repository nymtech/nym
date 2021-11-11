/* eslint-disable no-nested-ternary */
import * as React from 'react';
import { Link } from 'react-router-dom';
import { ChevronLeft, ExpandLess, ExpandMore, Menu } from '@mui/icons-material';
import { CSSObject, styled, Theme, useTheme } from '@mui/material/styles';
import MuiLink from '@mui/material/Link';
import Box from '@mui/material/Box';
import ListItem from '@mui/material/ListItem';
import MuiDrawer from '@mui/material/Drawer';
import AppBar from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import List from '@mui/material/List';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import { NymLogoSVG } from 'src/icons/NymLogoSVG';
import { BIG_DIPPER, NYM_WEBSITE } from 'src/api/constants';
import { useMediaQuery } from '@mui/material';
import { OverviewSVG } from '../icons/OverviewSVG';
import { NetworkComponentsSVG } from '../icons/NetworksSVG';
import { NodemapSVG } from '../icons/NodemapSVG';
import { Socials } from './Socials';
import { Footer } from './Footer';
import { DarkLightSwitchDesktop, DarkLightSwitchMobile } from './Switch';

const drawerWidth = 300;

const openedMixin = (theme: Theme): CSSObject => ({
  width: drawerWidth,
  transition: theme.transitions.create('width', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.enteringScreen,
  }),
  overflowX: 'hidden',
});

const closedMixin = (theme: Theme): CSSObject => ({
  transition: theme.transitions.create('width', {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  }),
  overflowX: 'hidden',
  width: `calc(${theme.spacing(7)} + 1px)`,
});

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  height: 64,
}));

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
}));

type navOptionType = {
  id: number;
  isActive?: boolean;
  url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: navOptionType[];
  isExpandedChild?: boolean;
};

const originalNavOptions: navOptionType[] = [
  {
    id: 0,
    isActive: false,
    url: '/overview',
    title: 'Overview',
    Icon: <OverviewSVG />,
  },
  {
    id: 1,
    isActive: false,
    url: '/network-components',
    title: 'Network Components',
    Icon: <NetworkComponentsSVG />,
    nested: [
      {
        id: 3,
        url: '/network-components/mixnodes',
        title: 'Mixnodes',
      },
      {
        id: 4,
        url: '/network-components/gateways',
        title: 'Gateways',
      },
      {
        id: 5,
        url: `${BIG_DIPPER}/validators`,
        title: 'Validators',
      },
    ],
  },
  {
    id: 2,
    isActive: false,
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <NodemapSVG />,
  },
];

type ExpandableButtonType = {
  id: number;
  title: string;
  url: string;
  isActive?: boolean;
  Icon?: React.ReactNode;
  nested?: navOptionType[];
  isChild?: boolean;
  openDrawer: () => void;
  closeDrawer: () => void;
  drawIsOpen: boolean;
  setToActive: (num: number) => void;
};

const ExpandableButton: React.FC<ExpandableButtonType> = ({
  id,
  url,
  setToActive,
  isActive,
  openDrawer,
  closeDrawer,
  drawIsOpen,
  Icon,
  title,
  nested,
  isChild,
}) => {
  const [dynamicStyle, setDynamicStyle] = React.useState({});
  const [nestedOptions, toggleNestedOptions] = React.useState(false);
  const [isExternal, setIsExternal] = React.useState<boolean>(false);
  const { palette } = useTheme();

  const handleClick = () => {
    openDrawer();
    if (title === 'Network Components' && nested) {
      toggleNestedOptions(!nestedOptions);
    }
    setToActive(id);
  };

  React.useEffect(() => {
    if (url) {
      setIsExternal(url.includes('http'));
    }
    if (nested) {
      setDynamicStyle({
        background: '#242C3D',
        borderRight: `3px solid ${palette.nym.highlight}`,
      });
    }
    if (isChild) {
      setDynamicStyle({
        background: '#3C4558',
        fontWeight: 800,
      });
    }
    if (!nested && !isChild) {
      setDynamicStyle({
        background: '#242C3D',
        borderRight: `3px solid ${palette.nym.highlight}`,
      });
    }
  }, [url]);

  React.useEffect(() => {
    if (!drawIsOpen && nestedOptions) {
      toggleNestedOptions(false);
    }
  }, [drawIsOpen]);

  return (
    <>
      <ListItem
        disablePadding
        disableGutters
        component={!nested ? Link : 'div'}
        to={isExternal ? { pathname: url } : url}
        target={isExternal ? '_blank' : ''}
        sx={
          isActive
            ? dynamicStyle
            : { background: '#111826', borderRight: 'none' }
        }
      >
        <ListItemButton
          onClick={handleClick}
          sx={{
            pt: 2,
            pb: 2,
            background: isChild ? '#3C4558' : 'none',
          }}
        >
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: palette.nym.networkExplorer.nav.text,
            }}
            primaryTypographyProps={{
              style: {
                fontWeight: isActive ? 800 : 300,
              },
            }}
          />
          {nested && nestedOptions && <ExpandLess />}
          {nested && !nestedOptions && <ExpandMore />}
        </ListItemButton>
      </ListItem>
      {nestedOptions &&
        nested?.map((each) => (
          <ExpandableButton
            id={each.id}
            url={each.url}
            key={each.title}
            title={each.title}
            data-testid={each.title}
            openDrawer={openDrawer}
            drawIsOpen={drawIsOpen}
            closeDrawer={closeDrawer}
            setToActive={setToActive}
            isChild
          />
        ))}
    </>
  );
};

ExpandableButton.defaultProps = {
  Icon: null,
  nested: undefined,
  isChild: false,
  isActive: false,
};

export const Nav: React.FC = ({ children }) => {
  const [navOptionsState, updateNavOptionsState] =
    React.useState(originalNavOptions);
  const [open, setOpen] = React.useState(false);
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));

  const setToActive = (id: number) => {
    const updated = navOptionsState.map((option) => ({
      ...option,
      isActive: option.id === id,
    }));
    updateNavOptionsState(updated);
  };

  const handleDrawerOpen = () => {
    setOpen(true);
  };

  const handleDrawerClose = () => {
    setOpen(false);
  };

  return (
    <>
      <Box sx={{ display: 'flex' }}>
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
            }}
          >
            <Box
              sx={{
                display: 'flex',
                flexDirection: 'row',
                alignItems: 'center',
                justifyContent: 'space-between',
                width: 205,
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
            <Box
              sx={{
                mr: 2,
                alignItems: 'center',
                display: 'flex',
              }}
            >
              {!isMobile && (
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
                  <Socials />
                  <DarkLightSwitchDesktop defaultChecked />
                </Box>
              )}
              {isMobile && <DarkLightSwitchMobile />}
            </Box>
          </Toolbar>
        </AppBar>
        <Drawer
          variant="permanent"
          open={open}
          sx={{
            background: theme.palette.nym.networkExplorer.nav.background,
          }}
        >
          <DrawerHeader
            sx={{
              justifyContent: open ? 'flex-end' : 'center',
              paddingLeft: 0,
            }}
          >
            <IconButton
              onClick={open ? handleDrawerClose : handleDrawerOpen}
              sx={{
                padding: 0,
                ml: '7px',
                color: theme.palette.nym.networkExplorer.nav.text,
              }}
              data-testid="menu-buttons"
            >
              {open ? <ChevronLeft /> : <Menu />}
            </IconButton>
          </DrawerHeader>

          <List sx={{ pt: 0, pb: 0 }}>
            {navOptionsState.map((props) => (
              <ExpandableButton
                setToActive={setToActive}
                key={props.url}
                openDrawer={handleDrawerOpen}
                drawIsOpen={open}
                closeDrawer={handleDrawerClose}
                {...props}
              />
            ))}
          </List>
        </Drawer>
        <Box sx={{ width: '100%', p: 4, mt: 7 }}>
          {children}
          <Footer />
        </Box>
      </Box>
    </>
  );
};
