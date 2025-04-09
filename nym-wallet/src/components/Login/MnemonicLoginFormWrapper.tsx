import React, { useRef, useEffect } from 'react';
import { Box } from '@mui/material';
import { MnemonicInput as OriginalMnemonicInput } from '@nymproject/react/textfields/Mnemonic';
import { readText } from '@tauri-apps/plugin-clipboard-manager';
import ContentPasteIcon from '@mui/icons-material/ContentPaste';
import IconButton from '@mui/material/IconButton';
import Tooltip from '@mui/material/Tooltip';

interface PasteButtonProps {
  onPaste: () => void;
}

const PasteButton: React.FC<PasteButtonProps> = ({ onPaste }) => (
  <Tooltip title="Paste from clipboard">
    <IconButton size="small" onClick={onPaste} aria-label="paste from clipboard">
      <ContentPasteIcon fontSize="small" />
    </IconButton>
  </Tooltip>
);

interface EnhancedMnemonicInputProps {
  mnemonic: string;
  onUpdateMnemonic: (mnemonic: string) => void;
  error?: string;
  [key: string]: any;
}

export { OriginalMnemonicInput as MnemonicInput };

export const EnhancedMnemonicInput: React.FC<EnhancedMnemonicInputProps> = ({
  mnemonic,
  onUpdateMnemonic,
  ...otherProps
}) => {
  const inputRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const findInputElement = () => {
      if (!inputRef.current) return undefined;

      const textarea = inputRef.current.querySelector('textarea');
      const input = textarea || inputRef.current.querySelector('input');

      if (!input) return undefined;

      // Fix the event type issue by casting Event to KeyboardEvent
      const handleKeyDown = async (e: Event) => {
        const keyEvent = e as KeyboardEvent;
        if (document.activeElement !== input) return;

        if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'a') {
          keyEvent.preventDefault();
          setTimeout(() => {
            if (textarea) {
              textarea.select();
            } else {
              (input as HTMLInputElement).select();
            }
          }, 0);
        }

        if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'v') {
          keyEvent.preventDefault();
          try {
            const clipboardText = await readText();
            if (clipboardText) {
              onUpdateMnemonic(clipboardText.trim());
            }
          } catch (err) {
            // eslint-disable-next-line no-console
            console.error('Failed to paste text:', err);
          }
        }
      };

      input.addEventListener('keydown', handleKeyDown);

      return () => {
        input.removeEventListener('keydown', handleKeyDown);
      };
    };

    const cleanup = findInputElement();
    const timeoutId = setTimeout(findInputElement, 100);

    return () => {
      if (cleanup) cleanup();
      clearTimeout(timeoutId);
    };
  }, [onUpdateMnemonic]);

  const handlePaste = async () => {
    try {
      const clipboardText = await readText();
      if (clipboardText) {
        onUpdateMnemonic(clipboardText.trim());

        const textarea = inputRef.current?.querySelector('textarea');
        const input = textarea || inputRef.current?.querySelector('input');
        if (input) {
          input.focus();
        }
      }
    } catch (err) {
      // eslint-disable-next-line no-console
      console.error('Failed to paste from clipboard:', err);
    }
  };

  return (
    <Box position="relative" ref={inputRef}>
      <OriginalMnemonicInput mnemonic={mnemonic} onUpdateMnemonic={onUpdateMnemonic} {...otherProps} />
      <Box
        sx={{
          position: 'absolute',
          right: '14px',
          top: '16px',
          zIndex: 1,
        }}
      >
        <PasteButton onPaste={handlePaste} />
      </Box>
    </Box>
  );
};
