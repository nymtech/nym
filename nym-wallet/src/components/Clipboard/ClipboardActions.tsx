import React, { useEffect, useState, useRef } from 'react';
import { Button, IconButton, Tooltip } from '@mui/material';
import { Check, ContentCopy, ContentPaste } from '@mui/icons-material';
import { writeText, readText } from '@tauri-apps/plugin-clipboard-manager';
import { Console } from '../../utils/console';

/**
 * ClipboardActions component handles copy and paste operations
 * Can be used as standalone buttons or as icons
 */
export const ClipboardActions = ({
  text = '',
  iconButton = false,
  showCopy = true,
  showPaste = true,
  onPaste,
  pasteTooltip = 'Paste',
  copyTooltip = 'Copy',
  onCopy,
  fieldRef,
}: {
  text?: string;
  iconButton?: boolean;
  showCopy?: boolean;
  showPaste?: boolean;
  onPaste?: (pastedText: string) => void;
  pasteTooltip?: string;
  copyTooltip?: string;
  onCopy?: () => void;
  fieldRef?: React.MutableRefObject<HTMLInputElement | HTMLTextAreaElement | null>;
}) => {
  const [copied, setCopied] = useState(false);
  const [pasted, setPasted] = useState(false);

  const handleCopy = async (_text: string) => {
    try {
      await writeText(_text);
      setCopied(true);
      onCopy?.();
    } catch (e) {
      Console.error(`failed to copy: ${e}`);
    }
  };

  const handlePaste = async () => {
    try {
      // Try Tauri clipboard first
      const pastedText = await readText();

      if (pastedText) {
        onPaste?.(pastedText);
        setPasted(true);
      }
    } catch (e) {
      // Fallback to browser clipboard
      try {
        const pastedText = await navigator.clipboard.readText();
        if (pastedText) {
          onPaste?.(pastedText);
          setPasted(true);
        }
      } catch (err) {
        Console.error(`paste failed: ${err}`);
      }
    }
  };

  const wasSelectAllPressed = useRef(false);

  useEffect(() => {
    if (!fieldRef) return;

    const keydownHandler = async (e: KeyboardEvent) => {
      // Only handle if the associated field is focused
      const { activeElement } = document;
      if (fieldRef.current && activeElement === fieldRef.current) {
        // Handle Select All (Cmd+A or Ctrl+A)
        if ((e.metaKey || e.ctrlKey) && e.key === 'a') {
          wasSelectAllPressed.current = true;
          return;
        }

        // Handle paste (Cmd+V or Ctrl+V)
        if ((e.metaKey || e.ctrlKey) && e.key === 'v' && onPaste) {
          e.preventDefault();
          await handlePaste();
          return;
        }

        if ((e.metaKey || e.ctrlKey) && e.key === 'c' && showCopy) {
          const field = fieldRef.current;

          const selectedText = field.value.substring(field.selectionStart || 0, field.selectionEnd || 0);

          if (selectedText || wasSelectAllPressed.current) {
            e.preventDefault();

            if (wasSelectAllPressed.current && !selectedText) {
              field.select();
              const textToCopy = field.value;
              await writeText(textToCopy);
            } else {
              await writeText(selectedText);
            }

            setCopied(true);
            onCopy?.();
            wasSelectAllPressed.current = false;
          } else if (text) {
            e.preventDefault();
            await writeText(text);
            setCopied(true);
            onCopy?.();
          }
        }
      }
    };

    // When field loses focus, reset the Select All tracking
    const blurHandler = () => {
      wasSelectAllPressed.current = false;
    };

    // Add keydown and blur event listeners
    document.addEventListener('keydown', keydownHandler);
    if (fieldRef.current) {
      fieldRef.current.addEventListener('blur', blurHandler);
    }

    // eslint-disable-next-line consistent-return
    return () => {
      document.removeEventListener('keydown', keydownHandler);
      if (fieldRef.current) {
        fieldRef.current.removeEventListener('blur', blurHandler);
      }
    };
  }, [onPaste, fieldRef, text, showCopy, onCopy]);

  useEffect(() => {
    const timer = setTimeout(() => {
      setCopied(false);
      setPasted(false);
    }, 2000);
    return () => clearTimeout(timer);
  }, [copied, pasted]);

  if (iconButton) {
    return (
      <div style={{ display: 'flex', alignItems: 'center' }}>
        {showCopy && (
          <Tooltip title={!copied ? copyTooltip : 'Copied!'} leaveDelay={500}>
            <IconButton
              onClick={() => handleCopy(text)}
              size="small"
              sx={{
                color: 'text.primary',
                mr: showPaste ? 0.5 : 0,
              }}
              disabled={!text}
            >
              {!copied ? <ContentCopy sx={{ fontSize: 14 }} /> : <Check color="success" sx={{ fontSize: 14 }} />}
            </IconButton>
          </Tooltip>
        )}

        {showPaste && (
          <Tooltip title={!pasted ? pasteTooltip : 'Pasted!'} leaveDelay={500}>
            <IconButton
              onClick={handlePaste}
              size="small"
              sx={{
                color: 'text.primary',
              }}
            >
              {!pasted ? <ContentPaste sx={{ fontSize: 14 }} /> : <Check color="success" sx={{ fontSize: 14 }} />}
            </IconButton>
          </Tooltip>
        )}
      </div>
    );
  }

  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
      {showCopy && (
        <Button
          variant="outlined"
          color="inherit"
          sx={{
            color: 'text.primary',
            borderColor: 'text.primary',
          }}
          onClick={() => handleCopy(text)}
          endIcon={copied && <Check sx={{ color: (theme) => theme.palette.success.light }} />}
          disabled={!text}
        >
          {!copied ? 'Copy' : 'Copied'}
        </Button>
      )}

      {showPaste && (
        <Button
          variant="outlined"
          color="inherit"
          sx={{
            color: 'text.primary',
            borderColor: 'text.primary',
          }}
          onClick={handlePaste}
          endIcon={pasted && <Check sx={{ color: (theme) => theme.palette.success.light }} />}
        >
          {!pasted ? 'Paste' : 'Pasted'}
        </Button>
      )}
    </div>
  );
};

// For backward compatibility
export const CopyToClipboard = ({
  text = '',
  iconButton,
  onPaste,
  fieldRef,
}: {
  text?: string;
  iconButton?: boolean;
  onPaste?: (pastedText: string) => void;
  fieldRef?: React.MutableRefObject<HTMLInputElement | HTMLTextAreaElement | null>;
}) => (
  <ClipboardActions text={text} iconButton={iconButton} onPaste={onPaste} showPaste={!!onPaste} fieldRef={fieldRef} />
);

// Export a paste-only variant for convenience
export const PasteFromClipboard = ({
  onPaste,
  iconButton = true,
  tooltip = 'Paste',
  fieldRef,
}: {
  onPaste: (pastedText: string) => void;
  iconButton?: boolean;
  tooltip?: string;
  fieldRef?: React.MutableRefObject<HTMLInputElement | HTMLTextAreaElement | null>;
}) => (
  <ClipboardActions
    iconButton={iconButton}
    showCopy={false}
    showPaste
    onPaste={onPaste}
    pasteTooltip={tooltip}
    fieldRef={fieldRef}
  />
);
