import React, { useState } from 'react';
import { Typography } from '@mui/material';
import { ActionsMenu, ActionsMenuItem } from 'src/components/ActionsMenu';
import { Unbond as UnbondIcon } from '../../svg-icons';

export type TBondedMixnodeActions = 'nodeSettings' | 'bondMore' | 'unbond' | 'redeem' | 'compound';

export const BondedMixnodeActions = ({
  onActionSelect,
  disabledRedeemAndCompound,
}: {
  onActionSelect: (action: TBondedMixnodeActions) => void;
  disabledRedeemAndCompound: boolean;
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
        title="Unbond"
        Icon={<UnbondIcon fontSize="inherit" />}
        onClick={() => handleActionClick('unbond')}
      />
      <ActionsMenuItem
        title="Compound rewards"
        Icon={<Typography sx={{ pl: 1 }}>C</Typography>}
        description={disabledRedeemAndCompound ? 'No rewards to compound' : 'Add your rewards to you balance'}
        onClick={() => handleActionClick('compound')}
        disabled={disabledRedeemAndCompound}
      />
      <ActionsMenuItem
        title="Redeem rewards"
        Icon={<Typography sx={{ pl: 1 }}>R</Typography>}
        description={disabledRedeemAndCompound ? 'No rewards to redeem' : 'Add your rewards to you balance'}
        onClick={() => handleActionClick('redeem')}
        disabled={disabledRedeemAndCompound}
      />
    </ActionsMenu>
  );
};
