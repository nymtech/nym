import React, { useState } from 'react';
import { Button } from '@mui/material';
import { SendModal } from './Sendmodal';

export const Send = () => {
  const [showModal, setShowModal] = useState(false);

  return (
    <>
      <Button variant="outlined" size="small" disableElevation onClick={() => setShowModal(true)}>
        Send
      </Button>
      {showModal && <SendModal onClose={() => setShowModal(false)} />}
    </>
  );
};
