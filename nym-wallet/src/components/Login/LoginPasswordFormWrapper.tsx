import React, { useRef, useEffect } from 'react';
import { Box } from '@mui/material';
import { PasswordInput } from '@nymproject/react/textfields/Password';
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

interface EnhancedPasswordInputProps {
  password: string;
  onUpdatePassword: (password: string) => void;
  label?: string;
  placeholder?: string;
  error?: string;
  autoFocus?: boolean;
  disabled?: boolean;
  [key: string]: any;
}

export const EnhancedPasswordInput: React.FC<EnhancedPasswordInputProps> = ({
  password,
  onUpdatePassword,
  ...otherProps
}) => {
  const inputRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const findInputElement = () => {
      if (!inputRef.current) return undefined;

      const input = inputRef.current.querySelector('input');
      if (!input) return undefined;

      const handleKeyDown = async (e: KeyboardEvent) => {
        if (document.activeElement !== input) return;

        if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
          e.preventDefault();
          setTimeout(() => {
            (input as HTMLInputElement).select();
          }, 0);
        }

        if ((e.metaKey || e.ctrlKey) && e.key === 'v') {
          e.preventDefault();
          try {
            const clipboardText = await readText();
            if (clipboardText) {
              onUpdatePassword(clipboardText);
            }
          } catch (err) {
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
  }, [onUpdatePassword]);

  const handlePaste = async () => {
    try {
      const clipboardText = await readText();
      if (clipboardText) {
        onUpdatePassword(clipboardText);

        const input = inputRef.current?.querySelector('input');
        if (input) {
          input.focus();
        }
      }
    } catch (err) {
      console.error('Failed to paste from clipboard:', err);
    }
  };

  return (
    <Box position="relative" ref={inputRef}>
      <PasswordInput password={password} onUpdatePassword={onUpdatePassword} {...otherProps} />
      <Box
        sx={{
          position: 'absolute',
          right: '40px',
          top: '50%',
          transform: 'translateY(-50%)',
          zIndex: 1,
        }}
      >
        <PasteButton onPaste={handlePaste} />
      </Box>
    </Box>
  );
};
