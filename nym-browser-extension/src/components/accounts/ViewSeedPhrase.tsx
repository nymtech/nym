import React, { useState } from 'react';
import { Box, Card, CardContent, Typography } from '@mui/material';
import { PasswordInput } from '@nymproject/react/textfields/Password';
import { ExtensionStorage } from '@nymproject/extension-storage';
import { Button, ConfirmationModal } from 'src/components/ui';

const ShowSeedButton = ({ handleShowSeedPhrase }: { handleShowSeedPhrase: () => void }) => (
  <Button fullWidth variant="contained" onClick={handleShowSeedPhrase}>
    Show seed phrase
  </Button>
);

const DoneButton = ({ onDone }: { onDone: () => void }) => (
  <Button fullWidth variant="contained" onClick={onDone}>
    Done
  </Button>
);

const Seed = ({ seed }: { seed: string }) => (
  <Card>
    <CardContent>
      <Typography>{seed}</Typography>
    </CardContent>
  </Card>
);

export const ViewSeedPhrase = ({ accountName, onDone }: { accountName: string; onDone: () => void }) => {
  const [seed, setSeed] = useState<string>();
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string>();

  const handleShowSeedPhrase = async () => {
    try {
      const storage = await new ExtensionStorage(password);
      const accountSeed = await storage.read_mnemonic(accountName);
      setSeed(accountSeed);
    } catch (e) {
      setError('Could not retrieve seed phrase. Please check your password');
    }
  };

  return (
    <ConfirmationModal
      open
      onClose={onDone}
      title={seed ? 'Account seed phrase' : 'Password'}
      subtitle={seed ? '' : 'Enter your account password'}
      ConfirmButton={
        seed ? <DoneButton onDone={onDone} /> : <ShowSeedButton handleShowSeedPhrase={handleShowSeedPhrase} />
      }
    >
      {seed ? (
        <Seed seed={seed} />
      ) : (
        <Box sx={{ mt: 2 }}>
          <PasswordInput
            label="Password"
            error={error}
            password={password}
            onUpdatePassword={(pw: string) => {
              setPassword(pw);
            }}
          />
        </Box>
      )}
    </ConfirmationModal>
  );
};
