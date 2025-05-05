import * as React from 'react';
import { useState } from 'react';
import { Stack, Typography, Box } from '@mui/material';
import { useBuyContext } from 'src/context';
import { SimpleModal } from '../Modals/SimpleModal';
import { ErrorModal } from '../Modals/ErrorModal';
import { TextFieldWithPaste } from '../Clipboard/ClipboardFormFields';
import { CopyToClipboard } from '../Clipboard/ClipboardActions';

export const SignMessageModal = ({ onClose }: { onClose: () => void }) => {
  const [message, setMessage] = useState<string>('');
  const [signature, setSignature] = useState<string>('');

  const { signMessage, loading, refresh, error } = useBuyContext();

  const handleSign = async () => {
    if (!message) {
      return;
    }
    try {
      const sig = await signMessage(message);
      setSignature(sig || '');
    } catch (err) {
      // eslint-disable-next-line no-console
      console.error('Signing failed:', err);
      setSignature('');
    }
  };

  const handleMessageChange = (value: string) => {
    setMessage(value);
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
    <SimpleModal
      open
      header="Sign message"
      okLabel="Sign"
      onOk={handleSign}
      onClose={onClose}
      okDisabled={loading || !message}
    >
      <Stack spacing={3} sx={{ width: '100%' }}>
        <Box>
          <Typography variant="caption" color="text.secondary" sx={{ mb: 1, display: 'block' }}>
            Message to Sign
          </Typography>
          <TextFieldWithPaste
            label=""
            multiline
            rows={8}
            placeholder="Enter or paste your message"
            fullWidth
            value={message}
            onPasteValue={handleMessageChange}
            onChange={(e) => handleMessageChange(e.target.value)}
            InputLabelProps={{ shrink: false }}
          />
        </Box>

        <Box>
          <Typography variant="caption" color="text.secondary" sx={{ mb: 1, display: 'block' }}>
            Signature
          </Typography>
          <Box sx={{ position: 'relative' }}>
            <TextFieldWithPaste
              label=""
              multiline
              rows={3}
              value={signature}
              placeholder="Signature will appear here"
              fullWidth
              disabled
              InputLabelProps={{ shrink: false }}
              InputProps={{
                readOnly: true,
                endAdornment: signature ? (
                  <Box sx={{ position: 'absolute', top: 8, right: 8 }}>
                    <CopyToClipboard text={signature} iconButton />
                  </Box>
                ) : undefined,
              }}
            />
          </Box>
        </Box>
      </Stack>
    </SimpleModal>
  );
};
