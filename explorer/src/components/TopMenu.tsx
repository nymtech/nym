import React, { ReactElement } from 'react';
import { Theme } from '@material-ui/core';
import { Telegram, Twitter } from '@material-ui/icons';
import { makeStyles } from '@material-ui/styles';

const useStyles = makeStyles((theme: Theme) => ({
  topMenu: {
    display: 'flex',
    height: 72,
    width: '100vw',
    padding: 24,
    alignItems: 'center',
    backgroundColor: '#070B15',
    position: 'absolute',
    justifyContent: 'space-between',
    top: 0,
    zIndex: 1,
  },
  colourCircle: {
    width: 45,
    height: 45,
    borderRadius: '50%',
    background: 'linear-gradient(90deg, #E1864B 0%, #DA465B 100%)',
    position: 'relative',
  },
  blackCircle: {
    width: 41,
    height: 41,
    borderRadius: '50%',
    background: '#070B15',
    position: 'absolute',
    left: 2,
    top: 2,
  },
  logoLetters: {
    color: 'white',
    position: 'absolute',
    left: 6,
    top: 12,
    fontSize: 12,
    margin: 0,
  },
  socialsMenu: {
    width: 200,
    height: '100%',
    display: 'flex',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-evenly',
  },
  socialsMenuItem: {
    borderRadius: '50%',
    background: 'white',
    height: 30,
    width: 30,
    display: 'flex',
    alignItems: 'center',
  },
  switch: {
    // backgroundColor: 'yellow',
  },
}));

const socialsRoutesSchema = [
  {
    label: 'Telegram',
    route: '/',
    Icon: (
      <Telegram
        style={{
          color: '#F2F2F2 !important',
          height: '90%',
          width: 'auto',
        }}
      />
    ),
  },
  {
    label: 'Twitter',
    route: '/',
    Icon: (
      <Twitter
        style={{
          color: '#F2F2F2 !important',
          height: '90%',
          width: 'auto',
          marginLeft: 2,
        }}
      />
    ),
  },
];

export default function TopMenu(): ReactElement {
  const classes = useStyles();
  return (
    <>
      <div className={classes.topMenu}>
        <div className={classes.colourCircle}>
          <div className={classes.blackCircle}>
            <h4 className={classes.logoLetters}>NYM</h4>
          </div>
        </div>
        <div className={classes.socialsMenu}>
          {socialsRoutesSchema.map(({ route, Icon }) => (
            <div className={classes.socialsMenuItem} key={route}>
              {Icon}
            </div>
          ))}
          <p style={{ color: 'white' }}>switch here</p>
        </div>
      </div>
    </>
  );
}
