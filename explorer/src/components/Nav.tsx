/* eslint-disable no-nested-ternary */
import * as React from 'react';
import { Link } from 'react-router-dom';
import { ChevronLeft, ExpandLess, ExpandMore } from '@mui/icons-material';
import { styled, CSSObject, Theme } from '@mui/material/styles';
import Box from '@mui/material/Box';
import ListItem from '@mui/material/ListItem';
import MuiDrawer from '@mui/material/Drawer';
import MuiAppBar, { AppBarProps as MuiAppBarProps } from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import List from '@mui/material/List';
import Typography from '@mui/material/Typography';
import IconButton from '@mui/material/IconButton';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import { NymLogoSVG } from 'src/icons/NymLogoSVG';
import { BIG_DIPPER } from 'src/api/constants';
import { useMediaQuery, useTheme } from '@mui/material';
import { OverviewSVG } from '../icons/OverviewSVG';
import { NetworkComponentsSVG } from '../icons/NetworksSVG';
import { NodemapSVG } from '../icons/NodemapSVG';
import { palette } from '../index';
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
  width: `calc(${theme.spacing(9)} + 1px)`,
  [theme.breakpoints.up('sm')]: {
    width: `calc(${theme.spacing(7)} + 1px)`,
  },
});

const DrawerHeader = styled('div')(({ theme }) => ({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'flex-end',
  padding: theme.spacing(0, 1),
  // necessary for content to be below app bar
  ...theme.mixins.toolbar,
}));
interface AppBarProps extends MuiAppBarProps {
  open?: boolean;
}

const AppBar = styled(MuiAppBar, {
  shouldForwardProp: (prop) => prop !== 'open',
})<AppBarProps>(({ theme, open }) => ({
  zIndex: theme.zIndex.drawer + 1,
  transition: theme.transitions.create(['width', 'margin'], {
    easing: theme.transitions.easing.sharp,
    duration: theme.transitions.duration.leavingScreen,
  }),
  ...(open && {
    marginLeft: drawerWidth,
    width: `calc(100% - ${drawerWidth}px)`,
    transition: theme.transitions.create(['width', 'margin'], {
      easing: theme.transitions.easing.sharp,
      duration: theme.transitions.duration.enteringScreen,
    }),
  }),
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

  const handleClick = () => {
    openDrawer();
    // 1. if it's NetworkComponents parent...
    if (title === 'Network Components') {
      if (nested) {
        // 2. if it contains nested routes
        // open them up
        toggleNestedOptions(!nestedOptions);
      }
    }
    // 3. if its NOT NetworkComponents and the drawer was open
    // close it, and implicitly this closes nest via useEffect below
    if (drawIsOpen && title !== 'Network Components') {
      closeDrawer();
    }
    // 4. update isActive in parent
    setToActive(id);
  };

  React.useEffect(() => {
    if (url) {
      setIsExternal(url.includes('http'));
    }
    if (nested) {
      setDynamicStyle({
        background: '#242C3D',
        borderRight: `3px solid ${palette.brandOrange}`,
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
        borderRight: `3px solid ${palette.brandOrange}`,
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
              color: (theme) => theme.palette.primary.main,
            }}
            primaryTypographyProps={{
              style: {
                fontWeight: isActive ? 800 : 300,
              },
            }}
          />
          {nested && nestedOptions && <ExpandLess color="primary" />}
          {nested && !nestedOptions && <ExpandMore color="primary" />}
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
    const newStuff = navOptionsState;

    const updated = newStuff.map((option) => {
      if (option.id === id) {
        return {
          ...option,
          isActive: true,
        };
      }
      return {
        ...option,
        isActive: false,
      };
    });
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
          position="fixed"
          open={open}
          sx={{
            background: theme.palette.primary.dark,
          }}
        >
          <Toolbar
            disableGutters
            sx={{
              paddingLeft: 2,
              display: 'flex',
              justifyContent: 'space-between',
            }}
          >
            <Box display="flex" alignItems="center">
              <IconButton
                color="inherit"
                aria-label="open drawer"
                onClick={handleDrawerOpen}
                edge="start"
                sx={{
                  ...(open && {
                    display: 'none',
                    margin: 0,
                    padding: 2,
                  }),
                }}
              >
                <NymLogoSVG />
              </IconButton>
              <Typography
                variant="h6"
                noWrap
                component="div"
                sx={{
                  marginLeft: 3,
                  color: theme.palette.primary.main,
                }}
                data-testid="nym-explorer-button"
              >
                Network Explorer
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
                  <Socials disableDarkMode />
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
            background: palette.blackBg,
          }}
        >
          <DrawerHeader sx={{ background: theme.palette.primary.dark }}>
            <IconButton onClick={handleDrawerClose}>
              <ChevronLeft color="primary" />
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
        <Box sx={{ width: '100%', p: 4 }}>
          <DrawerHeader />
          {children}
          <Footer />
        </Box>
      </Box>
    </>
  );
};
