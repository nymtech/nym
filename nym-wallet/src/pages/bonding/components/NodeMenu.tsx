import React, { useState } from 'react';
import { Typography } from '@mui/material';
import { ActionsMenu, ActionsMenuItem } from 'src/components/ActionsMenu';
import { Bond as BondIcon, Unbond as UnbondIcon } from '../../../svg-icons';
import { GatewayFlow } from '../gateway/types';
import { MixnodeFlow } from '../mixnode/types';

interface Item {
  label: string;
  flow: MixnodeFlow | GatewayFlow;
  icon: React.ReactNode;
  description?: string;
}

const NodeMenu = ({ onFlowChange }: { onFlowChange: (flow: MixnodeFlow) => void }) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpen = () => setIsOpen(true);
  const handleClose = () => setIsOpen(false);

  const handleActionClick = (flow: MixnodeFlow | GatewayFlow) => {
    onFlowChange(flow);
    handleClose();
  };

  return (
    <ActionsMenu open={isOpen} onOpen={handleOpen} onClose={handleClose}>
      <ActionsMenuItem
        title="Bond more"
        Icon={<BondIcon fontSize="inherit" />}
        onClick={() => handleActionClick('bondMore')}
      />
      <ActionsMenuItem
        title="Unbond"
        Icon={<UnbondIcon fontSize="inherit" />}
        onClick={() => handleActionClick('unbond')}
      />
      <ActionsMenuItem
        title="Compound rewards"
        Icon={<Typography sx={{ pl: 1 }}>C</Typography>}
        description="Add operator rewards to bond"
        onClick={() => handleActionClick('compound')}
      />
      <ActionsMenuItem
        title="Redeem rewards"
        Icon={<Typography sx={{ pl: 1 }}>R</Typography>}
        description="Add your rewards to bonding pool"
        onClick={() => handleActionClick('redeem')}
      />
    </ActionsMenu>
  );
};

export default NodeMenu;
