import React, { ReactElement } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { List, ListItem, ListItemIcon, Theme } from '@material-ui/core';
import { Equalizer, Menu, Close, GroupWork, PinDrop } from '@material-ui/icons';
import { makeStyles, ClassNameMap } from '@material-ui/styles';

const useStyles = makeStyles((theme: Theme) => ({
  navBar: {
    backgroundColor: '#242C3D',
    height: '100vh',
    padding: 24,
    width: 110,
    transition: '0.2s ease-in-out',
    display: 'flex',
    flexDirection: 'column',
  },
  navItem: {
    color: '#fff',
    fontWeight: 600,
    fontSize: 14,
    fontFamily: 'open sans, sans-serif',
    transition: '0.2s ease-out',
    animation: '$myEffect 1s ease-in',
  },
  selected: {
    color: theme.palette.primary.main,
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
export default function Nav(): ReactElement {
  const [sidebar, setSidebar] = React.useState<boolean>(false);
  const classes: ClassNameMap = useStyles();
  const location = useLocation();
  const showSidebar = () => setSidebar(!sidebar);

  return (
    <>
      <div
        className={classes.navBar}
        style={sidebar ? { width: 290 } : { width: 110 }}
      >
        <List>
          <ListItem button onClick={showSidebar} style={{ height: 72 }}>
            <ListItemIcon>
              {sidebar ? <Close /> : <Menu style={{ color: '#5C616D' }} />}
            </ListItemIcon>
          </ListItem>
          {routesSchema.map(({ route, Icon, label }) => (
            <ListItem button component={Link} to={route} style={{ height: 72 }}>
              <ListItemIcon>{Icon}</ListItemIcon>
              {sidebar && (
                <p
                  className={classes.navItem}
                  style={
                    location.pathname === route
                      ? { color: 'orange' }
                      : { color: '#F2F2F2' }
                  }
                >
                  {label}
                </p>
              )}
            </ListItem>
          ))}
        </List>
      </div>
    </>
  );
}
