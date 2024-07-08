import React, { useState } from 'react';
import { ActionsMenu, ActionsMenuItem } from '@src/components/ActionsMenu';
import { Unbond as UnbondIcon } from '../../svg-icons';

export type TBondedGatwayActions = 'unbond';

export const BondedGatewayActions = ({
  onActionSelect,
}: {
  onActionSelect: (action: TBondedGatwayActions) => void;
}) => {
  const [isOpen, setIsOpen] = useState(false);

  const handleOpen = () => setIsOpen(true);
  const handleClose = () => setIsOpen(false);

  const handleActionClick = (action: TBondedGatwayActions) => {
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
    </ActionsMenu>
  );
};
