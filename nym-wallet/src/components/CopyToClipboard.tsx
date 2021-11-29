import React, { useEffect, useState } from 'react'
import { IconButton, Tooltip } from '@mui/material'
import { Check, ContentCopy } from '@mui/icons-material'
import { clipboard } from '@tauri-apps/api'

export const CopyToClipboard = ({
  text = '',
  light,
}: {
  text?: string
  light?: boolean
}) => {
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
    <Tooltip title={!copied ? 'Copy' : 'Copied!'} leaveDelay={500}>
      <IconButton
        onClick={() => handleCopy(text)}
        size="small"
        sx={{
          color: (theme) =>
            light ? theme.palette.common.white : theme.palette.grey[600],
        }}
      >
        {!copied ? <ContentCopy fontSize="small" /> : <Check color="success" />}
      </IconButton>
    </Tooltip>
  )
}
