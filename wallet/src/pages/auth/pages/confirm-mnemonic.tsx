import { useContext, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Button, Stack } from '@mui/material';
import { validateMnemonic } from '@src/requests';
import { MnemonicInput } from '@nymproject/react';
import { AuthContext } from '@src/context/auth';
import { Subtitle } from '../components';

export const ConfirmMnemonic = () => {
  const { error, setError, setMnemonic, mnemonic } = useContext(AuthContext);
  const [localMnemonic, setLocalMnemonic] = useState(mnemonic);
  const navigate = useNavigate();

  useEffect(() => {
    setError(undefined);
  }, [localMnemonic]);

  return (
    <Stack spacing={2} sx={{ minWidth: '50%' }}>
      <Subtitle subtitle="Enter the mnemonic you wish to create a password for" />
      <MnemonicInput mnemonic={localMnemonic} onUpdateMnemonic={(mnc) => setLocalMnemonic(mnc)} error={error} />
      <Button
        size="large"
        variant="contained"
        fullWidth
        onClick={async () => {
          const isValid = await validateMnemonic(localMnemonic);
          if (isValid) {
            setMnemonic(localMnemonic);
            navigate('/connect-password');
          } else {
            setError('The mnemonic provided is not valid. Please check the mnemonic');
          }
        }}
        disabled={localMnemonic.length === 0}
      >
        Next
      </Button>
      <Button
        size="large"
        color="inherit"
        fullWidth
        onClick={() => {
          setMnemonic('');
          navigate(-1);
        }}
      >
        Back
      </Button>
    </Stack>
  );
};
