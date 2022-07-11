import * as React from 'react';
import { IconButton, Menu, MenuItem, Stack, Typography } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import { useEffect } from 'react';
import { MixnodeFlow } from '../mixnode/types';
import { GatewayFlow } from '../gateway/types';

interface Item {
  label: string;
  flow: MixnodeFlow | GatewayFlow;
  icon: React.ReactNode;
  description?: string;
}

interface Props {
  onFlowChange: (flow: MixnodeFlow | GatewayFlow) => void;
  items: Item[];
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
        {items.map(({ label, flow, icon, description }) => (
          <MenuItem onClick={() => onClick(flow)} key={flow} sx={{ px: 1.6 }} disableRipple>
            <Stack direction="row" spacing={1}>
              <Stack display="flex" alignItems="flex-end" width={16} alignSelf="start">
                {icon}
              </Stack>
              <Stack alignItems="flex-start" justifyContent="flex-start">
                <Typography>{label}</Typography>
                <Typography variant="subtitle2" color="nym.text.muted">
                  {description}
                </Typography>
              </Stack>
            </Stack>
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};

export default NodeMenu;
