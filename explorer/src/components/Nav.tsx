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
// import { makeStyles, ClassNameMap } from '@mui/material/styles';
import { makeStyles, ClassNameMap } from '@mui/styles';
// import { useTheme } from '@emotion/react';

const useStyles = makeStyles((theme: Theme) => {
  console.log('useStyles has a theme? ', theme);
  return {
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
      padding: '24px !important',
    },
    activeListItem: {
      backgroundColor: '#111826 !important',
    },
    navItem: {
      color: '#fff',
      fontWeight: 600,
      // fontSize: theme.typography.fontSize,
      // fontFamily: theme.typography.fontFamily,
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
  };
});

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
export const Nav: React.FC = (props) => {
  console.log('Nav props ', props);
  const [sidebar, setSidebar] = React.useState<boolean>(false);
  // const theme = useTheme();
  // console.log('theme iiiiissss ', theme);
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
