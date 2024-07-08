import { useState } from 'react';
import { Stack, TextField } from '@mui/material';
import FormControlLabel from '@mui/material/FormControlLabel';
import Checkbox from '@mui/material/Checkbox';
import { Error } from '../warnings/Error';

export type MnemonicInputProps = {
  mnemonic: string;
  error?: string;
  onUpdateMnemonic: (mnemonic: string) => void;
};

export const MnemonicInput = ({ mnemonic, error, onUpdateMnemonic }: MnemonicInputProps) => {
  const [showMnemonic, setShowMnemonic] = useState(false);
  return (
    <Stack spacing={2}>
      <TextField
        label="Mnemonic"
        type={showMnemonic ? 'input' : 'password'}
        value={mnemonic}
        onChange={(e) => onUpdateMnemonic(e.target.value)}
        autoFocus
        fullWidth
        multiline={showMnemonic}
        InputLabelProps={{ shrink: true }}
        sx={{
          'input::-webkit-textfield-decoration-container': {
            alignItems: 'start',
          },
        }}
      />
      <FormControlLabel
        control={<Checkbox checked={Boolean(showMnemonic)} onChange={() => setShowMnemonic((show) => !show)} />}
        label="Reveal my mnemonic"
      />
      {error && <Error message={error} />}
    </Stack>
  );
};
