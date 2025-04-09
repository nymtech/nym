import React, { useRef, useEffect } from 'react';
import { TextField, InputAdornment, Box, TextFieldProps } from '@mui/material';
import { CurrencyFormField } from '@nymproject/react/currency/CurrencyFormField';
import { CurrencyDenom, DecCoin } from '@nymproject/types';
import { UseFormRegister, UseFormSetValue, FieldValues, Path, FieldErrors } from 'react-hook-form';
import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
import { PasteFromClipboard } from './ClipboardActions';

export const useCopyAllSupport = (inputRef: React.MutableRefObject<HTMLInputElement | HTMLTextAreaElement | null>, onPasteValue?: (value: string) => void) => {
  useEffect(() => {
    if (!inputRef.current) return undefined;

    const handleKeyDown = async (e: Event) => {
      const keyEvent = e as KeyboardEvent;

      if (document.activeElement !== inputRef.current) return;

      // Handle Cmd+A or Ctrl+A (Select All)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'a') {
        setTimeout(() => {
          if (document.activeElement === inputRef.current && inputRef.current) {
            inputRef.current.select();
          }
        }, 0);
      }

      // Handle Cmd+C or Ctrl+C (Copy)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'c') {
        if (inputRef.current && inputRef.current.selectionStart !== inputRef.current.selectionEnd) {
          const selectedText = inputRef.current.value.substring(
            inputRef.current.selectionStart || 0,
            inputRef.current.selectionEnd || 0,
          );

          if (selectedText) {
            keyEvent.preventDefault();
            writeText(selectedText).catch((err) => {
              // eslint-disable-next-line no-console
              console.error('Failed to copy text:', err);
            });
          }
        }
      }

      // Handle Cmd+V or Ctrl+V (Paste)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'v' && onPasteValue) {
        try {
          keyEvent.preventDefault();
          const clipboardText = await readText();
          if (clipboardText) {
            onPasteValue(clipboardText);
          }
        } catch (err) {
          // eslint-disable-next-line no-console
          console.error('Failed to paste text:', err);
        }
      }
    };

    const input = inputRef.current;
    input.addEventListener('keydown', handleKeyDown);

    return () => {
      if (input) {
        input.removeEventListener('keydown', handleKeyDown);
      }
    };
  }, [inputRef.current, onPasteValue]);
};

export const TextFieldWithPaste = React.forwardRef<
  HTMLDivElement,
  TextFieldProps & {
    onPasteValue?: (value: string) => void;
  }
>(({ onPasteValue, ...props }, ref) => {
  const inputRef = useRef<HTMLInputElement>(null);

  useCopyAllSupport(inputRef, onPasteValue);

  const handlePaste = (pastedText: string) => {
    onPasteValue?.(pastedText);

    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  return (
    <TextField
      {...props}
      ref={ref}
      inputRef={inputRef}
      InputProps={{
        ...props.InputProps,
        endAdornment: (
          <InputAdornment position="end">
            {onPasteValue && <PasteFromClipboard onPaste={handlePaste} fieldRef={inputRef} />}
            {props.InputProps?.endAdornment}
          </InputAdornment>
        ),
      }}
    />
  );
});

// Add defaultProps to fix the "require-default-props" warning
TextFieldWithPaste.defaultProps = {
  onPasteValue: undefined,
};

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

  // Process pasted text to clean the value
  const processPastedText = (pastedText: string) => {
    if (!pastedText) return;

    let cleanedValue = pastedText.trim();

    // Remove non-numeric characters except decimal point
    cleanedValue = cleanedValue.replace(/[^\d.]/g, '');

    // Ensure only one decimal point
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
      inputRef.current.focus();
    }
  };

  // Modified to pass the processPastedText function to useCopyAllSupport
  useEffect(() => {
    if (!inputRef.current) return undefined;

    const handleKeyDown = async (e: Event) => {
      const keyEvent = e as KeyboardEvent;
      if (document.activeElement !== inputRef.current) return;

      // Handle Cmd+A (Select All)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'a' && inputRef.current) {
        setTimeout(() => {
          if (inputRef.current) inputRef.current.select();
        }, 0);
      }

      // Handle Cmd+C (Copy)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'c' && inputRef.current) {
        if (inputRef.current.selectionStart !== inputRef.current.selectionEnd) {
          const selectedText = inputRef.current.value.substring(
            inputRef.current.selectionStart || 0,
            inputRef.current.selectionEnd || 0,
          );

          if (selectedText) {
            keyEvent.preventDefault();
            writeText(selectedText).catch((err) => {
              // eslint-disable-next-line no-console
              console.error('Failed to copy text:', err);
            });
          }
        }
      }

      // Handle Cmd+V (Paste)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'v') {
        try {
          keyEvent.preventDefault();
          const clipboardText = await readText();
          if (clipboardText) {
            processPastedText(clipboardText);
          }
        } catch (err) {
          // eslint-disable-next-line no-console
          console.error('Failed to paste text:', err);
        }
      }
    };

    const input = inputRef.current;
    if (input) {
      input.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      if (input) {
        input.removeEventListener('keydown', handleKeyDown);
      }
    };
  }, [inputRef.current, denom, onChanged]);

  // Find the input element
  useEffect(() => {
    const findInputElement = () => {
      if (fieldRef.current) {
        inputRef.current = fieldRef.current.querySelector('input');
      }
    };

    findInputElement();
    const timeoutId = setTimeout(findInputElement, 100);

    return () => clearTimeout(timeoutId);
  }, []);

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

type CurrencyFieldError = {
  amount?: {
    message?: string;
  };
};

export const HookFormTextFieldWithPaste = <TFieldValues extends FieldValues>({
  name,
  label,
  register,
  setValue,
  errors,
  ...props
}: {
  name: Path<TFieldValues>;
  label: string;
  register: UseFormRegister<TFieldValues>;
  setValue: UseFormSetValue<TFieldValues>;
  errors: FieldErrors<TFieldValues>;
} & Omit<TextFieldProps, 'name' | 'label'>) => {
  const inputRef = useRef<HTMLInputElement>(null);

  const handlePaste = (pastedText: string) => {
    setValue(name, pastedText as any, { shouldValidate: true });

    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  // Pass handlePaste to useCopyAllSupport for Cmd+V handling
  useCopyAllSupport(inputRef, handlePaste);

  return (
    <TextField
      {...register(name)}
      name={name}
      label={label}
      error={Boolean(errors[name])}
      helperText={errors[name]?.message?.toString()}
      inputRef={inputRef}
      InputProps={{
        endAdornment: (
          <InputAdornment position="end">
            <PasteFromClipboard onPaste={handlePaste} fieldRef={inputRef} />
          </InputAdornment>
        ),
        ...props.InputProps,
      }}
      {...props}
    />
  );
};

export const HookFormCurrencyFieldWithPaste = <TFieldValues extends FieldValues>({
  name,
  label,
  setValue,
  errors,
  denom,
  initialValue,
  ...props
}: {
  name: Path<TFieldValues>;
  label: string;
  setValue: UseFormSetValue<TFieldValues>;
  errors: FieldErrors<TFieldValues>;
  denom?: CurrencyDenom;
  initialValue?: string;
}) => {
  const fieldRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement | null>(null);

  // Handle pasting with number formatting
  const handlePaste = (pastedText: string) => {
    if (!pastedText) return;

    let cleanedValue = pastedText.trim();

    cleanedValue = cleanedValue.replace(/[^\d.]/g, '');

    const parts = cleanedValue.split('.');
    if (parts.length > 2) {
      cleanedValue = `${parts[0]}.${parts.slice(1).join('')}`;
    }

    setValue(`${String(name)}.amount` as Path<TFieldValues>, cleanedValue as any, { shouldValidate: true });

    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  // Enable copy-all support for this field with Cmd+V handling
  useEffect(() => {
    if (!inputRef.current) return undefined;

    const handleKeyDown = async (e: Event) => {
      const keyEvent = e as KeyboardEvent;
      if (document.activeElement !== inputRef.current) return;

      // Handle Cmd+A (Select All)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'a' && inputRef.current) {
        setTimeout(() => {
          if (inputRef.current) inputRef.current.select();
        }, 0);
      }

      // Handle Cmd+C (Copy)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'c' && inputRef.current) {
        if (inputRef.current.selectionStart !== inputRef.current.selectionEnd) {
          const selectedText = inputRef.current.value.substring(
            inputRef.current.selectionStart || 0,
            inputRef.current.selectionEnd || 0,
          );

          if (selectedText) {
            keyEvent.preventDefault();
            writeText(selectedText).catch((err) => {
              // eslint-disable-next-line no-console
              console.error('Failed to copy text:', err);
            });
          }
        }
      }

      // Handle Cmd+V (Paste)
      if ((keyEvent.metaKey || keyEvent.ctrlKey) && keyEvent.key === 'v') {
        try {
          keyEvent.preventDefault();
          const clipboardText = await readText();
          if (clipboardText) {
            handlePaste(clipboardText);
          }
        } catch (err) {
          // eslint-disable-next-line no-console
          console.error('Failed to paste text:', err);
        }
      }
    };

    const input = inputRef.current;
    if (input) {
      input.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      if (input) {
        input.removeEventListener('keydown', handleKeyDown);
      }
    };
  }, [inputRef.current, name, setValue]);

  useEffect(() => {
    const findInputElement = () => {
      if (fieldRef.current) {
        inputRef.current = fieldRef.current.querySelector('input');
      }
    };

    findInputElement();
    const timeoutId = setTimeout(findInputElement, 100);

    return () => clearTimeout(timeoutId);
  }, []);

  // Safely access error message
  const getErrorMessage = (): string | undefined => {
    const fieldError = errors[name] as unknown as CurrencyFieldError | undefined;
    return fieldError?.amount?.message;
  };

  return (
    <Box position="relative" width="100%" ref={fieldRef}>
      <CurrencyFormField
        label={label}
        fullWidth
        onChanged={(value) => {
          setValue(`${String(name)}.amount` as Path<TFieldValues>, value.amount as any, { shouldValidate: true });
        }}
        initialValue={initialValue}
        denom={denom}
        validationError={getErrorMessage()}
        {...props}
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
        <PasteFromClipboard onPaste={handlePaste} fieldRef={inputRef} />
      </Box>
    </Box>
  );
};