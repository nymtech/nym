import React, { useRef, useEffect } from 'react';
import { Box } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { PasteFromClipboard } from './Clipboard/ClipboardActions';

export const CurrencyFormFieldWithPaste = ({
  label,
  fullWidth,
  onChanged,
  initialValue,
  denom,
  required,
  autoFocus,
  validationError,
}: {
  label: string;
  fullWidth?: boolean;
  onChanged: (value: DecCoin) => void;
  initialValue?: string;
  denom?: CurrencyDenom;
  required?: boolean;
  autoFocus?: boolean;
  validationError?: string;
}) => {
  const fieldRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  const processPastedText = (pastedText: string) => {
    if (!pastedText) return;

    let cleanedValue = pastedText.trim();
    cleanedValue = cleanedValue.replace(/[^\d.]/g, '');

    const parts = cleanedValue.split('.');
    if (parts.length > 2) {
      cleanedValue = `${parts[0]}.${parts.slice(1).join('')}`;
    }

    const decCoin: DecCoin = {
      amount: cleanedValue,
      denom: denom as any,
    };

    onChanged(decCoin);

    if (inputRef.current) {
      inputRef.current.value = cleanedValue;

      const inputEvent = new Event('input', { bubbles: true });
      inputRef.current.dispatchEvent(inputEvent);

      const changeEvent = new Event('change', { bubbles: true });
      inputRef.current.dispatchEvent(changeEvent);

      inputRef.current.focus();
    }
  };

  useEffect(() => {
    const pasteEventHandler = (e: ClipboardEvent) => {
      e.preventDefault();

      const { clipboardData } = e;
      if (!clipboardData) return;

      const pastedText = clipboardData.getData('text');
      if (!pastedText) return;

      processPastedText(pastedText);
    };

    const findInputElement = () => {
      if (fieldRef.current) {
        const input = fieldRef.current.querySelector('input');
        if (input) {
          inputRef.current = input;

          // Set up paste event handler
          input.addEventListener('paste', pasteEventHandler as EventListener);
        }
      }
    };

    findInputElement();
    const timeoutId = setTimeout(findInputElement, 200);

    return () => {
      clearTimeout(timeoutId);
      if (inputRef.current) {
        inputRef.current.removeEventListener('paste', pasteEventHandler as EventListener);
      }
    };
  }, [denom, onChanged]);

  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      if (inputRef.current && document.activeElement === inputRef.current) {
        if ((e.metaKey || e.ctrlKey) && e.key === 'v') {
          e.preventDefault();

          try {
            const clipboardText = await navigator.clipboard.readText();
            if (clipboardText) {
              processPastedText(clipboardText);
            }
          } catch (err) {
            // eslint-disable-next-line no-console
            console.error('Error accessing clipboard:', err);
          }
        }
      }
    };

    document.addEventListener('keydown', handleKeyDown);

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [denom, onChanged]);

  return (
    <Box position="relative" width="100%" ref={fieldRef}>
      <CurrencyFormField
        label={label}
        fullWidth={fullWidth}
        onChanged={onChanged}
        initialValue={initialValue}
        denom={denom}
        required={required}
        autoFocus={autoFocus}
        validationError={validationError}
      />
      <Box
        sx={{
          position: 'absolute',
          right: '14px',
          top: '50%',
          transform: 'translateY(-50%)',
          zIndex: 1,
        }}
      >
        <PasteFromClipboard onPaste={processPastedText} fieldRef={inputRef} />
      </Box>
    </Box>
  );
};
