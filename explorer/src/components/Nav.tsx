import * as React from 'react';
import { styled, CSSObject, Theme } from '@mui/material/styles';
import { Link, useLocation } from 'react-router-dom';
// MUI surfaces etc
import Box from '@mui/material/Box';
import MuiDrawer from '@mui/material/Drawer';
import MuiAppBar, { AppBarProps as MuiAppBarProps } from '@mui/material/AppBar';
import Toolbar from '@mui/material/Toolbar';
import List from '@mui/material/List';
import Typography from '@mui/material/Typography';
import Divider from '@mui/material/Divider';
import IconButton from '@mui/material/IconButton';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
// MUI Icons
import MenuIcon from '@mui/icons-material/Menu';
import ChevronLeftIcon from '@mui/icons-material/ChevronLeft';
import BarChartIcon from '@mui/icons-material/BarChart';
import ConnectIcon from '@mui/icons-material/CastConnected';
import PinIcon from '@mui/icons-material/PinDropOutlined';
import ExpandLess from '@mui/icons-material/ExpandLess';
import ExpandMore from '@mui/icons-material/ExpandMore';
// non-MUI icons
// import { theme } from 'src/theme';
import { NymLogoSVG } from '../icons/NymLogoSVG';

const drawerWidth = 270;

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
  [theme.breakpoints.up('sm')]: {
    width: `calc(${theme.spacing(9)} + 1px)`,
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

interface NavigationListItemButtonProps {
  isSelected?: boolean;
  to?: string;
  component?: React.ReactNode;
}

const NavigationListItemButton = styled(ListItemButton, {
  shouldForwardProp: (prop) => prop !== 'isSelected',
})<NavigationListItemButtonProps>(({ theme, isSelected }) => ({
  backgroundColor: isSelected
    ? theme.palette.primary.dark
    : theme.palette.primary.light,
}));

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
  url: string;
  title: string;
  icon: any;
  nested?: any;
};

type navOptionsType = navOptionType[];

const navOptions: navOptionsType = [
  // {
  //   url: '/',
  //   title: 'Home',
  //   icon: HomeIcon,
  // },
  {
    url: '/overview',
    title: 'Overview',
    icon: BarChartIcon,
  },
  {
    url: '/network-components',
    title: 'Network Components',
    icon: ConnectIcon,
    nested: [
      {
        url: '/network-components/mixnodes',
        title: 'Mixnodes',
      },
      {
        url: '/network-components/gateways',
        title: 'Gateways',
      },
    ],
  },
  {
    url: '/nodemap',
    title: 'Nodemap',
    icon: PinIcon,
  },
];

const ExpandableButton = ({
  // expandable,
  nested,
  to,
  isSelected,
  title,
  SVGIcon,
}: any) => {
  const [open, toggle] = React.useState(false);

  const handleClick = () => toggle(!open);

  if (!nested)
    return (
      <NavigationListItemButton
        isSelected={isSelected}
        to={to}
        component={Link}
      >
        <ListItemIcon>
          <SVGIcon />
        </ListItemIcon>
        <ListItemText
          primary={title}
          sx={{
            color: isSelected ? 'orange' : 'white',
          }}
        />
      </NavigationListItemButton>
    );
  return (
    <>
      <NavigationListItemButton onClick={handleClick}>
        <ListItemIcon>
          <SVGIcon />
        </ListItemIcon>
        <ListItemText
          primary={`${title} `}
          sx={{
            color: isSelected
              ? (theme) => theme.palette.primary.contrastText
              : (theme) => theme.palette.primary.main,
          }}
        />
        {open ? <ExpandLess /> : <ExpandMore />}
      </NavigationListItemButton>
      {open &&
        nested.map((each: any) => (
          <NavigationListItemButton
            key={each.url}
            to={each.url}
            component={Link}
            sx={{
              paddingLeft: (theme) => theme.spacing(9),
              bgcolor: '#3C4558',
            }}
          >
            <ListItemText
              primary={each.title}
              sx={{
                color: isSelected ? 'orange' : 'white',
              }}
            />
          </NavigationListItemButton>
        ))}
    </>
  );
};

export const Nav: React.FC = ({ children }) => {
  const [open, setOpen] = React.useState(false);
  const [page, setCurrentPage] = React.useState('/');
  const location = useLocation();

  const handleDrawerOpen = () => {
    setOpen(true);
  };

  const handleDrawerClose = () => {
    setOpen(false);
  };

  React.useEffect(() => {
    setCurrentPage(location.pathname);
  }, [location]);

  return (
    <Box sx={{ display: 'flex' }}>
      <AppBar position="fixed" open={open}>
        <Toolbar>
          <IconButton
            color="inherit"
            aria-label="open drawer"
            onClick={handleDrawerOpen}
            edge="start"
            sx={{
              marginRight: '36px',
              ...(open && { display: 'none' }),
            }}
          >
            <MenuIcon />
          </IconButton>
          <NymLogoSVG />
          <Typography variant="h6" noWrap component="div">
            Network Explorer
          </Typography>
        </Toolbar>
      </AppBar>
      <Drawer variant="permanent" open={open}>
        <DrawerHeader>
          <IconButton onClick={handleDrawerClose}>
            <ChevronLeftIcon />
          </IconButton>
        </DrawerHeader>
        <Divider />
        <List>
          {navOptions.map((route) => (
            <ExpandableButton
              isSelected={route.url === page}
              key={route.url}
              to={route.url}
              title={route.title}
              nested={route.nested}
              SVGIcon={route.icon}
            />
          ))}
        </List>
        <Divider />
      </Drawer>
      {children}
    </Box>
  );
};
