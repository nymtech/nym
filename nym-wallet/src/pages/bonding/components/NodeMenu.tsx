import * as React from 'react';
import { IconButton, Menu, MenuItem, Stack } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { useEffect } from 'react';
import { MixnodeFlow } from '../mixnode/types';
import { GatewayFlow } from '../gateway/types';

interface Props {
  onFlowChange: (flow: MixnodeFlow | GatewayFlow) => void;
  items: { label: string; flow: MixnodeFlow | GatewayFlow; icon: React.ReactNode }[];
  onOpen: (open: boolean) => void;
}

const NodeMenu = ({ onFlowChange, items, onOpen }: Props) => {
  const [menuAnchorEl, setMenuAnchorEl] = React.useState<null | HTMLElement>(null);
  const menuOpen = Boolean(menuAnchorEl);

  useEffect(() => {
    onOpen(menuOpen);
  }, [menuOpen]);

  const handleMenuClose = () => {
    setMenuAnchorEl(null);
  };

  const onClick = (flow: MixnodeFlow | GatewayFlow) => {
    onFlowChange(flow);
    handleMenuClose();
  };

  return (
    <>
      <IconButton
        sx={{ fontSize: '1rem', padding: 0 }}
        id="menu-button"
        onClick={(event) => setMenuAnchorEl(event.currentTarget)}
        aria-controls={menuOpen ? 'node-menu' : undefined}
        aria-haspopup="true"
        aria-expanded={menuOpen ? 'true' : undefined}
        disableTouchRipple
        disableFocusRipple
        disableRipple
      >
        <MoreVert fontSize="inherit" sx={{ color: menuOpen ? 'primary.main' : 'text.primary' }} />
      </IconButton>
      <Menu
        open={menuOpen}
        anchorEl={menuAnchorEl}
        onClose={handleMenuClose}
        id="node-menu"
        sx={{
          '& .MuiPaper-root': {
            borderRadius: '4px',
          },
        }}
      >
        {items.map(({ label, flow, icon }) => (
          <MenuItem onClick={() => onClick(flow)} key={flow} disableRipple>
            <Stack direction="row" spacing={2} gap={1}>
              {icon}
              {label}
            </Stack>
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};

export default NodeMenu;
