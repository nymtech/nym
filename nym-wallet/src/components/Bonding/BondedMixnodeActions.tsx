import React, { useState } from 'react';
import { Typography } from '@mui/material';
import { ActionsMenu, ActionsMenuItem } from 'src/components/ActionsMenu';
import { Unbond as UnbondIcon, Bond as BondIcon } from '../../svg-icons';

export type TBondedMixnodeActions = 'nodeSettings' | 'bondMore' | 'unbond' | 'redeem';

export const BondedMixnodeActions = ({
  onActionSelect,
  disabledRedeemAndCompound,
  disabledBondMore,
}: {
  onActionSelect: (action: TBondedMixnodeActions) => void;
  disabledRedeemAndCompound: boolean;
  disabledBondMore?: boolean;
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
      {!disabledBondMore && (
        <ActionsMenuItem
          title="Bond More"
          Icon={<BondIcon fontSize="inherit" />}
          onClick={() => handleActionClick('bondMore')}
        />
      )}
      <ActionsMenuItem
        title="Redeem rewards"
        Icon={<Typography sx={{ pl: 1, fontWeight: 700 }}>R</Typography>}
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
