import React, { useState, useContext } from 'react';
import { Button, List, ListItem, ListItemIcon, ListItemText, ListSubheader, Popover } from '@mui/material';
import { ArrowDropDown, CheckSharp } from '@mui/icons-material';
import { AppContext, IS_DEV_MODE } from '../context/main';
import { Network } from '../types';

const networks: { networkName: Network; name: string }[] = [
  { networkName: 'MAINNET', name: 'Nym Mainnet' },
  { networkName: 'SANDBOX', name: 'Testnet Sandbox' },
  { networkName: 'QA', name: 'QA' },
];

const NetworkItem: React.FC<{ title: string; isSelected: boolean; onSelect: () => void }> = ({
  title,
  isSelected,
  onSelect,
}) => (
  <ListItem button onClick={onSelect}>
    <ListItemIcon>{isSelected && <CheckSharp color="success" />}</ListItemIcon>
    <ListItemText>{title}</ListItemText>
  </ListItem>
);

export const NetworkSelector = () => {
  const { network, switchNetwork } = useContext(AppContext);

  const [anchorEl, setAnchorEl] = useState<HTMLButtonElement | null>(null);

  const handleClick = (event: React.MouseEvent<HTMLButtonElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  return (
    <>
      <Button
        variant="text"
        color="primary"
        sx={{ color: (theme) => `${theme.palette.nym.background.dark}` }}
        onClick={handleClick}
        disableElevation
        endIcon={<ArrowDropDown sx={{ color: (theme) => `1px solid ${theme.palette.nym.background.dark}` }} />}
      >
        {networks.find((n) => n.networkName === network)?.name}
      </Button>
      <Popover
        open={Boolean(anchorEl)}
        anchorEl={anchorEl}
        anchorOrigin={{
          vertical: 'bottom',
          horizontal: 'left',
        }}
        onClose={handleClose}
      >
        <List>
          <ListSubheader>Network selection</ListSubheader>
          {networks
            .filter(({ networkName }) => !(!IS_DEV_MODE && networkName === 'QA'))
            .map(({ name, networkName }) => (
              <NetworkItem
                key={networkName}
                title={name}
                isSelected={networkName === network}
                onSelect={() => {
                  handleClose();
                  switchNetwork(networkName);
                }}
              />
            ))}
        </List>
      </Popover>
    </>
  );
};
