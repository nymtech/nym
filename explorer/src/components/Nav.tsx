import React, { ReactElement } from 'react';
import { Link, useLocation } from 'react-router-dom';
import { List, ListItem, ListItemIcon, Theme } from '@material-ui/core';
import { Equalizer, Menu, Close, GroupWork, PinDrop } from '@material-ui/icons';
import { makeStyles, ClassNameMap } from '@material-ui/styles';

const useStyles = makeStyles((theme: Theme) => ({
  navBar: {
    backgroundColor: '#242C3D',
    marginTop: 60,
    height: '100vh',
    width: 80,
    transition: '0.2s ease-in-out',
    display: 'flex',
    flexDirection: 'column',
  },
  hamburgerIcon: {
    color: '#5C616D',
  },
  navListItem: {
    borderTop: 0.4,
    height: '72px',
    // borderStyle: 'solid !important',
    // borderWidth: '0.1px !important',
    // borderColor: 'rgba(255, 255, 255, 0.2) !important',
    // borderLeft: 'none !important',
    padding: '24px !important',
  },
  activeListItem: {
    backgroundColor: '#111826 !important',
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
                <Menu className={classes.hamburgerIcon} />
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
}
