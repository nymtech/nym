import React, { useState, useContext } from 'react';
import { Button, List, ListItemButton, ListItemIcon, ListItemText, ListSubheader, Popover, Stack } from '@mui/material';
import { ArrowDropDown, Check } from '@mui/icons-material';
import { Network } from '@src/types';
import { AppContext } from '../context/main';
import { config } from '../config';

const networks: { networkName: Network; name: string }[] = [
  { networkName: 'MAINNET', name: 'Nym Mainnet' },
  { networkName: 'SANDBOX', name: 'Testnet Sandbox' },
  { networkName: 'QA', name: 'QA' },
];

const NetworkItem: FCWithChildren<{ title: string; isSelected: boolean; onSelect: () => void }> = ({
  title,
  isSelected,
  onSelect,
}) => (
  <ListItemButton
    onClick={onSelect}
    sx={{
      minWidth: '180px',
      '&:hover': {
        backgroundColor: isSelected ? 'rgba(251, 110, 78, 0.08) !important' : undefined,
      },
    }}
  >
    <Stack direction="row" justifyContent="space-between" alignItems="center" gap={2} width="100%">
      <ListItemText
        primaryTypographyProps={{
          color: isSelected ? 'primary' : undefined,
        }}
        primary={title}
      />
      {isSelected && (
        <ListItemIcon sx={{ justifyContent: 'flex-end' }}>
          <Check color="primary" fontSize="small" />
        </ListItemIcon>
      )}
    </Stack>
  </ListItemButton>
);

export const NetworkSelector = () => {
  const { network, switchNetwork, appEnv } = useContext(AppContext);

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
        color="inherit"
        sx={{ color: 'text.primary', fontSize: 14 }}
        onClick={handleClick}
        disableElevation
        endIcon={<ArrowDropDown sx={{ color: (theme) => `1px solid ${theme.palette.text.primary}` }} />}
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
          <ListSubheader sx={{ backgroundColor: 'transparent' }}>Network selection</ListSubheader>
          {networks
            .filter(({ networkName }) => {
              // show all networks when in dev more or the user wants QA mode enabled
              if (config.IS_DEV_MODE || appEnv?.ENABLE_QA_MODE) {
                return true;
              }
              // otherwise, filter out QA
              return networkName !== 'QA';
            })
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
