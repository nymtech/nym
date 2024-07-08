import * as React from 'react';
import { useState } from 'react';
import { Stack, TextField, Typography } from '@mui/material';
import { useBuyContext } from '@src/context';
import { SimpleModal } from '../Modals/SimpleModal';
import { ErrorModal } from '../Modals/ErrorModal';
import { CopyToClipboard } from '../CopyToClipboard';

export const SignMessageModal = ({ onClose }: { onClose: () => void }) => {
  const [message, setMessage] = useState<string>();
  const [signature, setSignature] = useState<string>();

  const { signMessage, loading, refresh, error } = useBuyContext();

  const handleSign = async () => {
    if (!message) {
      return;
    }
    signMessage(message).then((sig) => {
      setSignature(sig);
    });
  };

  const handleChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    setMessage(event.target.value);
  };

  if (error) {
    return (
      <ErrorModal
        open
        message={`An error occured: ${error}`}
        onClose={() => {
          refresh();
        }}
      />
    );
  }

  return (
    <SimpleModal open header="Sign message" okLabel="Sign" onOk={handleSign} onClose={onClose} okDisabled={loading}>
      <Stack gap={2}>
        <TextField
          id="outlined-multiline-static"
          label="Message"
          multiline
          rows={8}
          placeholder="Paste your message here"
          fullWidth
          value={message}
          onChange={handleChange}
        />
        <TextField
          id="outlined-multiline-static"
          multiline
          rows={3}
          value={signature}
          placeholder="Signature"
          fullWidth
          disabled
        />

        <Stack direction="row" alignItems="center" alignSelf="flex-end">
          <Typography variant="body2" component="span" fontWeight={600} sx={{ mr: 1, color: 'text.primary' }}>
            Copy signature
          </Typography>
          <CopyToClipboard text={signature} iconButton />
        </Stack>
      </Stack>
    </SimpleModal>
  );
};
