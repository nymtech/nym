import React, { useContext, useState } from 'react';
import { Logout, MenuSharp } from '@mui/icons-material';
import { Menu, IconButton, MenuItem, ListItemIcon, Typography } from '@mui/material';
import { Node } from 'src/svg-icons/node';
import { AppContext } from 'src/context';

export const AppMenu = () => {
  const { logOut, handleShowSettings } = useContext(AppContext);

  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  return (
    <>
      <IconButton onClick={handleClick}>
        <MenuSharp />
      </IconButton>
      <Menu
        open={open}
        onClose={handleClose}
        anchorEl={anchorEl}
        anchorOrigin={{
          vertical: 'bottom',
          horizontal: 'right',
        }}
        transformOrigin={{
          vertical: 'top',
          horizontal: 'right',
        }}
      >
        <MenuItem
          onClick={() => {
            handleClose();
            handleShowSettings();
          }}
        >
          <ListItemIcon>
            <Node />
          </ListItemIcon>
          <Typography>Node settings</Typography>
        </MenuItem>
        <MenuItem
          onClick={() => {
            handleClose();
            logOut();
          }}
        >
          <ListItemIcon>
            <Logout />
          </ListItemIcon>
          <Typography>Log out</Typography>
        </MenuItem>
      </Menu>
    </>
  );
};
