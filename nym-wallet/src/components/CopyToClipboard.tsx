import React, { useEffect, useState } from 'react';
import { Button, IconButton, Tooltip } from '@mui/material';
import { Check, ContentCopy } from '@mui/icons-material';
import { clipboard } from '@tauri-apps/api';
import { Console } from '../utils/console';

export const CopyToClipboard = ({ text = '', iconButton }: { text?: string; iconButton?: boolean }) => {
  const [copied, setCopied] = useState(false);

  const handleCopy = async (_text: string) => {
    try {
      await clipboard.writeText(_text);
      setCopied(true);
    } catch (e) {
      Console.error(`failed to copy: ${e}`);
    }
  };

  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (copied) {
      timer = setTimeout(() => {
        setCopied(false);
      }, 2000);
    }
    return () => clearTimeout(timer);
  }, [copied]);

  if (iconButton)
    return (
      <Tooltip title={!copied ? 'Copy' : 'Copied!'} leaveDelay={500}>
        <IconButton
          onClick={() => handleCopy(text)}
          size="small"
          sx={{
            color: 'text.primary',
          }}
        >
          {!copied ? <ContentCopy sx={{ fontSize: 14 }} /> : <Check color="success" sx={{ fontSize: 14 }} />}
        </IconButton>
      </Tooltip>
    );

  return (
    <Button
      variant="outlined"
      color="inherit"
      sx={{
        color: 'text.primary',
        borderColor: 'text.primary',
      }}
      onClick={() => handleCopy(text)}
      endIcon={copied && <Check sx={{ color: (theme) => theme.palette.success.light }} />}
    >
      {!copied ? 'Copy' : 'Copied'}
    </Button>
  );
};
