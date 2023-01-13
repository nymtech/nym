import React, { useRef } from 'react';
import { MoreVertSharp } from '@mui/icons-material';
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem } from '@mui/material';

export const ActionsMenu: FCWithChildren<{
  open: boolean;
  children: React.ReactNode;
  onOpen: () => void;
  onClose: () => void;
}> = ({ children, open, onOpen, onClose }) => {
  const anchorEl: any = useRef<HTMLElement>();

  return (
    <>
      <IconButton ref={anchorEl} onClick={onOpen}>
        <MoreVertSharp sx={{ color: (t) => t.palette.nym.nymWallet.text.main }} />
      </IconButton>
      <Menu anchorEl={anchorEl.current} open={open} onClose={onClose}>
        {children}
      </Menu>
    </>
  );
};

export const ActionsMenuItem = ({
  title,
  description,
  onClick,
  Icon,
  disabled,
}: {
  title: string;
  description?: string;
  onClick?: () => void;
  Icon?: React.ReactNode;
  disabled?: boolean;
}) => (
  <MenuItem sx={{ p: 2 }} onClick={onClick} disabled={disabled}>
    <ListItemIcon sx={{ color: 'text.primary' }}>{Icon}</ListItemIcon>
    <ListItemText sx={{ color: 'text.primary' }} primary={title} secondary={description} />
  </MenuItem>
);
