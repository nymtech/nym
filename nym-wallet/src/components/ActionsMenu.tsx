import React, { useRef } from 'react';
import { MoreVertSharp } from '@mui/icons-material';
import { IconButton, ListItemIcon, ListItemText, Menu, MenuItem } from '@mui/material';

export const ActionsMenu: React.FC<{ open: boolean; onOpen: () => void; onClose: () => void }> = ({
  children,
  open,
  onOpen,
  onClose,
}) => {
  const anchorEl: any = useRef<HTMLElement>();

  return (
    <>
      <IconButton ref={anchorEl} onClick={onOpen}>
        <MoreVertSharp />
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
}) => {
  return (
    <MenuItem sx={{ p: 2 }} onClick={onClick} disabled={disabled}>
      <ListItemIcon sx={{ color: 'text.primary' }}>{Icon}</ListItemIcon>
      <ListItemText sx={{ color: 'text.primary' }} primary={title} secondary={description} />
    </MenuItem>
  );
};
