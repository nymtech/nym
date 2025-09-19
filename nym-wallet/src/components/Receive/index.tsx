import React, { useContext } from 'react';
import { AppContext } from 'src/context';
import { ReceiveModal } from './ReceiveModal';

export const Receive = () => {
  const { showReceiveModal, handleShowReceiveModal } = useContext(AppContext);

  if (showReceiveModal) return <ReceiveModal onClose={handleShowReceiveModal} />;

  return null;
};
