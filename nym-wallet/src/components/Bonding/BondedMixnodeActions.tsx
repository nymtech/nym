import React, { useState } from 'react';
import { Typography } from '@mui/material';
import { ActionsMenu, ActionsMenuItem } from 'src/components/ActionsMenu';
import { Bond as BondIcon, Unbond as UnbondIcon } from '../../svg-icons';

export type TBondedMixnodeActions = 'nodeSettings' | 'bondMore' | 'unbond' | 'compound' | 'redeem' | null;

export const BondedMixnodeActions = ({
  onActionSelect,
}: {
  onActionSelect: (action: TBondedMixnodeActions) => void;
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpen = () => setIsOpen(true);
  const handleClose = () => setIsOpen(false);

  const handleActionClick = (action: TBondedMixnodeActions) => {
    onActionSelect(action);
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
