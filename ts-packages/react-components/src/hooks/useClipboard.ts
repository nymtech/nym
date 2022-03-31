import { useState } from 'react';
import { useClipboard } from 'use-clipboard-copy';

export const useClipboardHook = () => {
  const [isCopied, setIsCopied] = useState(false);

  const clipboard = useClipboard();

  const copy = (value: string) => {
    clipboard.copy(value);
    setIsCopied(true);
  };

  return { isCopied, copy };
};
