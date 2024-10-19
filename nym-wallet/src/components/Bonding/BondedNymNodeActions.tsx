import React, { useState } from 'react';
import { Typography } from '@mui/material';
import { ActionsMenu, ActionsMenuItem } from 'src/components/ActionsMenu';
import { Unbond as UnbondIcon, Bond as BondIcon } from '../../svg-icons';

export type TBondedNymNodeActions = 'nodeSettings' | 'updateBond' | 'unbond' | 'redeem';

export const BondedNymNodeActions = ({
  onActionSelect,
  disabledRedeemAndCompound,
  disabledUpdateBond,
}: {
  onActionSelect: (action: TBondedNymNodeActions) => void;
  disabledRedeemAndCompound: boolean;
  disabledUpdateBond?: boolean;
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpen = () => setIsOpen(true);
  const handleClose = () => setIsOpen(false);

  const handleActionClick = (action: TBondedNymNodeActions) => {
    onActionSelect(action);
    handleClose();
  };

  return (
    <ActionsMenu open={isOpen} onOpen={handleOpen} onClose={handleClose}>
      {!disabledUpdateBond && (
        <ActionsMenuItem
          title="Change bond amount"
          Icon={<BondIcon fontSize="inherit" />}
          onClick={() => handleActionClick('updateBond')}
        />
      )}
      <ActionsMenuItem
        title="Redeem rewards"
        Icon={<Typography sx={{ pl: 0.5, fontWeight: 700 }}>R</Typography>}
        onClick={() => handleActionClick('redeem')}
        disabled={disabledRedeemAndCompound}
      />
      <ActionsMenuItem
        title="Unbond"
        Icon={<UnbondIcon fontSize="inherit" />}
        onClick={() => handleActionClick('unbond')}
      />
    </ActionsMenu>
  );
};
