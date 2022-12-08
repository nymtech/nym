import React, { useState } from 'react';
import { Button } from '@mui/material';
import { MockBuyContextProvider } from 'src/context/mocks/buy';

import { SignMessageModal } from './SignMessageModal';

export default {
  title: 'Buy/SignMessage',
  component: SignMessageModal,
};

export const SignMessage = () => {
  const [open, setOpen] = useState(false);

  return (
    <MockBuyContextProvider>
      <Button
        variant="contained"
        color="primary"
        onClick={() => {
          setOpen(true);
        }}
      >
        Open Sign Modal
      </Button>
      {open && (
        <SignMessageModal
          onClose={() => {
            setOpen(false);
          }}
        />
      )}
    </MockBuyContextProvider>
  );
};
