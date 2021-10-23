/* eslint-disable no-nested-ternary */
import * as React from 'react';
import { Link, useLocation } from 'react-router-dom';
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
import { useMediaQuery, useTheme } from '@mui/material';
import { BIG_DIPPER } from 'src/api/constants';
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
  url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: navOptionType[];
  isExpandedChild?: boolean;
};

const originalNavOptions: navOptionType[] = [
  {
    id: 0,
    url: '/overview',
    title: 'Overview',
    Icon: <OverviewSVG />,
  },
  {
    id: 1,
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
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <NodemapSVG />,
  },
];

type ExpandableButtonType = {
  //   id: number;
  //   url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: navOptionType[];
  //   isExpandedChild?: boolean;
  openDrawer: () => void;
  drawIsOpen: boolean;
};
const ExpandableButton: React.FC<ExpandableButtonType> = ({
  openDrawer,
  drawIsOpen,
  Icon,
  title,
  nested,
}) => {
  const [nestedOptions, toggleNestedOptions] = React.useState(false);

  const handleClick = () => {
    openDrawer();
    if (title === 'Network Components') {
      if (nested) {
        toggleNestedOptions(!nestedOptions);
      }
    }
  };

  React.useEffect(() => {
    if (!drawIsOpen && nestedOptions) {
      toggleNestedOptions(false);
    }
  }, [drawIsOpen]);
  return (
    <>
      <ListItem disablePadding disableGutters>
        <ListItemButton onClick={handleClick} sx={{ pt: 2, pb: 2 }}>
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: (theme) => theme.palette.primary.main,
            }}
            primaryTypographyProps={{
              style: {
                fontWeight: 300,
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
            key={each.title}
            title={each.title}
            openDrawer={openDrawer}
            drawIsOpen={drawIsOpen}
          />
        ))}
    </>
  );
};

ExpandableButton.defaultProps = {
  Icon: null,
  nested: undefined,
  //   isExpandedChild: false,
};

export const NewNav: React.FC = ({ children }) => {
  const [open, setOpen] = React.useState(true);
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
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
            background: 'darkblue',
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
                  <Socials hoverEffect disableDarkMode />
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
            background: theme.palette.secondary.dark,
          }}
        >
          <DrawerHeader sx={{ background: theme.palette.primary.dark }}>
            <IconButton onClick={handleDrawerClose}>
              <ChevronLeft color="primary" />
            </IconButton>
          </DrawerHeader>

          <List sx={{ pt: 0, pb: 0 }}>
            {originalNavOptions.map((props) => (
              <ExpandableButton
                key={props.url}
                openDrawer={handleDrawerOpen}
                drawIsOpen={open}
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

NewNav.defaultProps = {};
