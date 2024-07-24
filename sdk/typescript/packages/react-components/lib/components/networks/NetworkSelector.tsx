import { useState } from 'react';
import { Button, List, ListItem, ListItemIcon, ListItemText, ListSubheader, Popover } from '@mui/material';
import { ArrowDropDown, CheckSharp } from '@mui/icons-material';

// TODO: move to sensible shared location
export type Network = 'QA' | 'SANDBOX' | 'MAINNET';

const networks: { networkName: Network; name: string }[] = [
  { networkName: 'MAINNET', name: 'Nym Mainnet' },
  { networkName: 'SANDBOX', name: 'Testnet Sandbox' },
  { networkName: 'QA', name: 'QA' },
];

type NetworkItemProps = { title: string; isSelected: boolean; onSelect: () => void };

const NetworkItem = ({ title, isSelected, onSelect }: NetworkItemProps) => (
  <ListItem button onClick={onSelect}>
    <ListItemIcon>{isSelected && <CheckSharp color="success" />}</ListItemIcon>
    <ListItemText>{title}</ListItemText>
  </ListItem>
);

export type NetworkSelectorProps = {
  network?: Network;
  onSwitchNetwork?: (newNetwork: Network) => void;
};

export const NetworkSelector = ({ network, onSwitchNetwork }: NetworkSelectorProps) => {
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
        sx={{ color: (theme) => `${theme.palette.text.primary}` }}
        onClick={handleClick}
        disableElevation
        endIcon={<ArrowDropDown sx={{ color: (theme) => `1px solid ${theme.palette.primary.main}` }} />}
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
            .filter((n) => !(n.networkName === 'QA'))
            .map(({ name, networkName }) => (
              <NetworkItem
                key={networkName}
                title={name}
                isSelected={networkName === network}
                onSelect={() => {
                  handleClose();
                  if (onSwitchNetwork) {
                    onSwitchNetwork(networkName);
                  }
                }}
              />
            ))}
        </List>
      </Popover>
    </>
  );
};
