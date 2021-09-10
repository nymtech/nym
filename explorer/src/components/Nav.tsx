import React from 'react';
import { Link, useLocation } from 'react-router-dom';
import { List, ListItem, ListItemIcon, Theme } from '@mui/material';
import {
  Equalizer,
  Close,
  GroupWork,
  PinDrop,
  Menu as MenuIcon,
} from '@mui/icons-material';
import { makeStyles, ClassNameMap } from '@mui/styles';

const useStyles = makeStyles((theme: Theme) => ({
  navBar: {
    backgroundColor: theme.palette.secondary.dark,
    marginTop: 60,
    height: '100vh',
    width: 80,
    transition: `${theme.transitions.duration.short}ms`,
    display: 'flex',
    flexDirection: 'column',
  },
  hamburgerIcon: {
    color: theme.palette.secondary.light,
  },
  navListItem: {
    borderTop: 0.4,
    height: '72px',
    padding: '24px !important',
  },
  activeListItem: {
    backgroundColor: theme.palette.primary.dark,
  },
  navItem: {
    color: theme.palette.primary.main,
    fontWeight: theme.typography.fontWeightBold,
    fontSize: theme.typography.fontSize,
    fontFamily: theme.typography.fontFamily,
    transition: `${theme.transitions.duration.short}ms`,
    animation: '$myEffect 1s ease-in',
  },
  selected: {
    color: theme.palette.primary.contrastText,
  },
  '@keyframes myEffect': {
    '0%': {
      opacity: 0,
    },
    '100%': {
      opacity: 1,
    },
  },
  '@keyframes myEffectExit': {
    '0%': {
      opacity: 1,
    },
    '100%': {
      opacity: 0,
    },
  },
}));

const routesSchema = [
  {
    label: 'Overview',
    route: '/overview',
    Icon: <Equalizer style={{ color: '#F2F2F2' }} />,
  },
  {
    label: 'Network Components',
    route: '/network-components',
    Icon: <GroupWork style={{ color: '#F2F2F2' }} />,
  },
  {
    label: 'Node Map',
    route: '/nodemap',
    Icon: <PinDrop style={{ color: '#F2F2F2' }} />,
  },
];
export const Nav: React.FC = () => {
  const [sidebar, setSidebar] = React.useState<boolean>(false);
  const classes: ClassNameMap = useStyles();
  const location = useLocation();
  const showSidebar = () => setSidebar(!sidebar);

  return (
    <>
      <div
        className={classes.navBar}
        style={sidebar ? { width: 290 } : { width: 80 }}
      >
        <List>
          <ListItem
            button
            className={classes.navListItem}
            onClick={showSidebar}
            style={{ height: 72 }}
          >
            <ListItemIcon>
              {sidebar ? (
                <Close className={classes.hamburgerIcon} />
              ) : (
                <MenuIcon className={classes.hamburgerIcon} />
              )}
            </ListItemIcon>
          </ListItem>
          {routesSchema.map(({ route, Icon, label }) => {
            const isSelected: boolean = location.pathname === route;
            return (
              <React.Fragment key={route}>
                <ListItem
                  key={route}
                  button
                  component={Link}
                  to={route}
                  className={`${classes.navListItem} ${
                    isSelected && classes.activeListItem
                  }`}
                >
                  <ListItemIcon>{Icon}</ListItemIcon>
                  {sidebar && (
                    <p
                      className={`${classes.navItem} ${
                        isSelected && classes.selected
                      }`}
                    >
                      {label}
                    </p>
                  )}
                </ListItem>
              </React.Fragment>
            );
          })}
        </List>
      </div>
    </>
  );
};
