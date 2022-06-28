import * as React from 'react';
import { IconButton, Menu, MenuItem } from '@mui/material';
import { MoreVert } from '@mui/icons-material';
import EditIcon from '@mui/icons-material/Edit';
import { MixnodeFlow } from './types';

const NodeMenu = ({ onFlowChange }: { onFlowChange: (flow: MixnodeFlow) => void }) => {
  const [menuAnchorEl, setMenuAnchorEl] = React.useState<null | HTMLElement>(null);
  const menuOpen = Boolean(menuAnchorEl);

  const handleMenuClose = () => {
    setMenuAnchorEl(null);
  };

  const onClick = (flow: MixnodeFlow) => {
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
      >
        <MoreVert fontSize="inherit" sx={{ color: 'text.primary' }} />
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
        <MenuItem onClick={() => onClick('bondMore')} disableRipple>
          <EditIcon fontSize="inherit" />
          Bond more
        </MenuItem>
        <MenuItem onClick={() => onClick('unbound')} disableRipple>
          <EditIcon fontSize="inherit" />
          Unbond
        </MenuItem>
        <MenuItem onClick={() => onClick('compound')} disableRipple>
          <EditIcon fontSize="inherit" />
          Compound rewards
        </MenuItem>
        <MenuItem onClick={() => onClick('redeem')} disableRipple>
          <EditIcon fontSize="inherit" />
          Redeem rewards
        </MenuItem>
      </Menu>
    </>
  );
};

export default NodeMenu;
