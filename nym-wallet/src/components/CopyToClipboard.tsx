import React, { useState } from 'react'
import { Button } from '@mui/material'
import { Check } from '@mui/icons-material'
import { clipboard } from '@tauri-apps/api'

export const CopyToClipboard = ({ text }: { text: string }) => {
  const [copied, setCopied] = useState(false)

  const handleCopy = async (text: string) => {
    try {
      await clipboard.writeText(text)
      setCopied(true)
    } catch (e) {
      console.log('failed to copy: ' + e)
    }
  }

  return (
    <Button
      size="small"
      variant="outlined"
      color={copied ? 'success' : 'inherit'}
      aria-label="save"
      data-testid="copy-button"
      onClick={() => handleCopy(text)}
      endIcon={copied && <Check />}
    >
      {copied ? 'Copied' : 'Copy'}
    </Button>
  )
}
