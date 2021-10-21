import * as React from 'react';
import { Link, useLocation } from 'react-router-dom';
import {
  ChevronLeft,
  ExpandLess,
  ExpandMore,
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
import IconButton from '@mui/material/IconButton';
import ListItemButton from '@mui/material/ListItemButton';
import ListItemIcon from '@mui/material/ListItemIcon';
import ListItemText from '@mui/material/ListItemText';
import { NymLogoSVG } from 'src/icons/NymLogoSVG';
import { BIG_DIPPER } from 'src/api/constants';
import { OverviewSVG } from '../icons/OverviewSVG';
import { NetworkComponentsSVG } from '../icons/NetworksSVG';
import { NodemapSVG } from '../icons/NodemapSVG';
import { MainContext } from '../context/main';
import { palette } from '../index';

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
  id: number;
  url: string;
  title: string;
  Icon?: React.ReactNode;
  nested?: navOptionType[];
  isExpandedChild?: boolean;
  openDrawer: () => void;
  drawIsOpen: boolean;
};
const ExpandableButton: React.FC<ExpandableButtonType> = ({
  nested,
  title,
  Icon,
  url,
  isExpandedChild,
  openDrawer,
  drawIsOpen,
}: ExpandableButtonType) => {
  const [open, toggle] = React.useState(false);
  const [isExternal, setIsExternal] = React.useState<boolean>(false);
  const location = useLocation();

  const handleClickNoNestedItems = () => {
    openDrawer();
  };

  const handleClick = () => {
    openDrawer();
    if (nested) {
      toggle(!open);
    }
  };

  React.useEffect(() => {
    if (url) {
      setIsExternal(url.includes('http'));
    }
  }, [url]);

  React.useEffect(() => {
    if (!drawIsOpen && open) {
      toggle(false);
    }
  }, [drawIsOpen]);

  if (!nested) {
    return (
      <ListItem
        disableGutters
        component={Link}
        to={isExternal ? { pathname: url } : url}
        target={isExternal ? '_blank' : ''}
        disablePadding
        sx={{
          background: (theme) =>
            isExpandedChild
              ? palette.nested
              : location.pathname.includes(url)
              ? palette.selectedNotNested
              : theme.palette.secondary.dark,
          borderRight: location.pathname.includes(url)
            ? `3px solid ${palette.brandOrange}`
            : 'none',
          borderBottom: `1px solid ${palette.divider}`,
        }}
      >
        <ListItemButton
          onClick={handleClickNoNestedItems}
          sx={{ pt: 2, pb: 2 }}
        >
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: (theme) => theme.palette.primary.main,
            }}
            primaryTypographyProps={{
              style: {
                fontWeight: location.pathname.includes(url) ? 800 : 300,
              },
            }}
          />
        </ListItemButton>
      </ListItem>
    );
  }
  return (
    <>
      <ListItem
        disablePadding
        disableGutters
        sx={{
          background: (theme) =>
            open
              ? palette.selectedNotNested
              : isExpandedChild
              ? palette.nested
              : theme.palette.secondary.dark,
          borderRight: open ? `3px solid ${palette.brandOrange}` : 'none',
          borderBottom: `1px solid ${palette.divider}`,
        }}
      >
        <ListItemButton onClick={handleClick} sx={{ pt: 2, pb: 2 }}>
          <ListItemIcon>{Icon}</ListItemIcon>
          <ListItemText
            primary={title}
            sx={{
              color: (theme) => theme.palette.primary.main,
              fontWeight: location.pathname.includes(url) ? 800 : 300,
            }}
          />
          {open ? (
            <ExpandLess color="primary" />
          ) : (
            <ExpandMore color="primary" />
          )}
        </ListItemButton>
      </ListItem>
      {open &&
        nested?.map((each: navOptionType) => (
          <ExpandableButton
            key={each.title}
            isExpandedChild
            openDrawer={openDrawer}
            drawIsOpen={drawIsOpen}
            {...each}
          />
        ))}
    </>
  );
};

ExpandableButton.defaultProps = {
  Icon: null,
  nested: undefined,
  isExpandedChild: false,
};

export const Nav: React.FC = ({ children }) => {
  const { toggleMode, mode } = React.useContext(MainContext);
  const [open, setOpen] = React.useState(true);

  const handleDrawerOpen = () => {
    setOpen(true);
  };

  const handleDrawerClose = () => {
    setOpen(false);
  };

  return (
    <Box sx={{ display: 'flex' }}>
      <AppBar
        position="fixed"
        open={open}
        sx={{
          background: (theme) => theme.palette.primary.dark,
        }}
      >
        <Toolbar disableGutters sx={{ paddingLeft: 2 }}>
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
              color: (theme) => theme.palette.primary.main,
            }}
          >
            Network Explorer
          </Typography>
        </Toolbar>
      </AppBar>
      <Drawer
        variant="permanent"
        open={open}
        sx={{
          background: (theme) => theme.palette.secondary.dark,
        }}
      >
        <DrawerHeader
          sx={{ background: (theme) => theme.palette.primary.dark }}
        >
          <IconButton onClick={handleDrawerClose}>
            <ChevronLeft color="primary" />
          </IconButton>
        </DrawerHeader>

        <List sx={{ pt: 0, pb: 0 }}>
          {originalNavOptions.map((props) => (
            <ExpandableButton
              key={props.id}
              openDrawer={handleDrawerOpen}
              drawIsOpen={open}
              {...props}
            />
          ))}
        </List>
        <ListItem disableGutters disablePadding>
          <ListItemButton onClick={toggleMode} sx={{ pt: 2, pb: 2 }}>
            <ListItemIcon>
              {mode === 'light' ? <Brightness4Sharp /> : <WbSunnySharp />}
            </ListItemIcon>
            <ListItemText sx={{ color: (theme) => theme.palette.primary.main }}>
              {mode === 'light' ? 'Dark mode' : 'Light mode'}
            </ListItemText>
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

Nav.defaultProps = {};
