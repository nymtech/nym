import React, { useEffect, useState } from 'react'
import { IconButton, Tooltip } from '@mui/material'
import { Check, ContentCopy } from '@mui/icons-material'
import { clipboard } from '@tauri-apps/api'

export const CopyToClipboard = ({ text = '' }: { text?: string }) => {
  const [copied, setCopied] = useState(false)

  const handleCopy = async (text: string) => {
    try {
      await clipboard.writeText(text)
      setCopied(true)
    } catch (e) {
      console.log('failed to copy: ' + e)
    }
  }

  useEffect(() => {
    if (copied) {
      setTimeout(() => {
        setCopied(false)
      }, 1000)
    }
  }, [copied])
  return (
    <Tooltip title={!copied ? 'Copy' : 'Copied!'}>
      <IconButton onClick={() => handleCopy(text)} size="small">
        {!copied ? <ContentCopy fontSize="small" /> : <Check color="success" />}
      </IconButton>
    </Tooltip>
  )
}
