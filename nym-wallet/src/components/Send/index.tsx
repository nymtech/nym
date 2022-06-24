import React, { useContext, useState } from 'react';
import { SendModal } from './SendModal';
import { AppContext } from 'src/context';

export const Send = () => {
  const { showSendModal, handleShowSendModal } = useContext(AppContext);

  if (showSendModal) return <SendModal onClose={handleShowSendModal} />;

  return null;
};
