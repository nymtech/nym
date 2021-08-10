import React, { useState } from 'react'
import { Button, CircularProgress, useTheme } from '@material-ui/core'
import { Check } from '@material-ui/icons'

const copy = (text: string): Promise<{ success: boolean; value: string }> => {
  return new Promise((resolve, reject) => {
    navigator.clipboard
      .writeText(text)
      .then(() => resolve({ success: true, value: text }))
      .catch(() => reject({ success: false, value: 'Failed to copy' }))
  })
}

export const CopyToClipboard = ({ text }) => {
  const [copied, setCopied] = useState(false)
  const [isCopying, setIsCopying] = useState(false)

  const theme = useTheme()

  return (
    <Button
      variant="outlined"
      size="small"
      onClick={async () => {
        setIsCopying(true)
        setCopied(false)

        const res = await copy(text)

        if (res.success) {
          setCopied(true)
        } else {
          console.log('Unable to copy to clipboard')
        }
        setIsCopying(false)
      }}
      endIcon={
        copied ? (
          <Check style={{ color: theme.palette.success.dark }} />
        ) : isCopying ? (
          <CircularProgress
            size={15}
            style={{ color: theme.palette.success.dark }}
          />
        ) : null
      }
    >
      {!copied ? 'Copy' : 'Copied'}
    </Button>
  )
}
