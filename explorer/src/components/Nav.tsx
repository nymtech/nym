import * as React from 'react';
import { Link } from 'react-router-dom';
import {
  GarageTwoTone,
  TrafficSharp,
  Menu,
  ChevronLeft,
  BarChart,
  ExpandLess,
  ExpandMore,
  Pin,
  ConnectedTv,
  WbSunnySharp,
  Brightness4Sharp,
} from '@mui/icons-material';
import { styled, CSSObject, Theme } from '@mui/material/styles';
import Box from '@mui/material/Box';
import ListItem from '@mui/material/ListItem';
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
import { NymLogoSVG } from 'src/icons/NymLogoSVG';
import { MainContext } from '../context/main';

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
  url: string;
  title: string;
  Icon: React.ReactNode;
  // eslint-disable-next-line react/require-default-props
  nested?: navOptionType[];
};

const navOptions: navOptionType[] = [
  {
    url: '/overview',
    title: 'Overview',
    Icon: <BarChart />,
  },
  {
    url: '/network-components',
    title: 'Network Components',
    Icon: <ConnectedTv />,
    nested: [
      {
        url: '/network-components/mixnodes',
        title: 'Mixnodes',
        Icon: <TrafficSharp />,
      },
      {
        url: '/network-components/gateways',
        title: 'Gateways',
        Icon: <GarageTwoTone />,
      },
    ],
  },
  {
    url: '/nodemap',
    title: 'Nodemap',
    Icon: <Pin />,
  },
];

const ExpandableButton: React.FC<navOptionType> = ({
  nested,
  title,
  Icon,
  url,
}) => {
  const [open, toggle] = React.useState(false);
  const handleClick = () => toggle(!open);

  if (!nested)
    return (
      <ListItem disableGutters component={Link} to={url}>
        <ListItemButton
          sx={{
            color: (theme) =>
              theme.palette.mode === 'light' ? '#000' : '#fff',
          }}
        >
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: (theme) =>
                theme.palette.mode === 'light' ? '#000' : '#fff',
            }}
          />
        </ListItemButton>
      </ListItem>
    );
  return (
    <>
      <ListItem disableGutters>
        <ListItemButton
          onClick={handleClick}
          sx={{ color: "text.primary" }}
        >
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText primary={title} />
          {open ? <ExpandLess /> : <ExpandMore />}
        </ListItemButton>
      </ListItem>
      {open &&
        nested.map((each: navOptionType) => (
          <ListItem
            disableGutters
            key={each.url}
            component={Link}
            to={each.url}
          >
            <ListItemButton sx={{ color: "text.primary" }}>
              <ListItemIcon>{each.Icon}</ListItemIcon>
              <ListItemText
                primary={each.title}
                sx={{
                  color: (theme) =>
                    theme.palette.mode === 'light' ? '#000' : '#fff',
                }}
              />
            </ListItemButton>
          </ListItem>
        ))}
    </>
  );
};

export const Nav: React.FC = ({ children }) => {
  const { toggleMode, mode }: any = React.useContext(MainContext);

  const [open, setOpen] = React.useState(false);

  const handleDrawerOpen = () => {
    setOpen(true);
  };

  const handleDrawerClose = () => {
    setOpen(false);
  };

  return (
    <Box sx={{ display: 'flex' }}>
      <AppBar position="fixed" open={open} color="default">
        <Toolbar>
          <IconButton
            color="inherit"
            aria-label="open drawer"
            onClick={handleDrawerOpen}
            edge="start"
            sx={{
              ...(open && { display: 'none' }),
            }}
          >
            <Menu />
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
            <ChevronLeft />
          </IconButton>
        </DrawerHeader>
        <Divider />
        <List>
          {navOptions.map((route) => (
            <ExpandableButton key={route.url} {...route} />
          ))}
        </List>
        <Divider />
        <ListItem disableGutters>
          <ListItemButton onClick={toggleMode}>
            <ListItemIcon>
              {mode === 'light' ? <Brightness4Sharp /> : <WbSunnySharp />}
            </ListItemIcon>
            <ListItemText>Light</ListItemText>
          </ListItemButton>
        </ListItem>
      </Drawer>
      <Box sx={{ width: '100%', p: 4 }}>
        <DrawerHeader />
        {children}
      </Box>
    </Box>
  );
};
