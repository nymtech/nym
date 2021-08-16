import React, { useState } from 'react'
import { Button } from '@material-ui/core'
import { Check } from '@material-ui/icons'
import { green } from '@material-ui/core/colors'

const copy = (text: string): Promise<{ success: boolean; value: string }> => {
  return new Promise((resolve, reject) => {
    navigator.clipboard
      .writeText(text)
      .then(() => resolve({ success: true, value: text }))
      .catch((e) => reject({ success: false, value: 'Failed to copy: ' + e }))
  })
}

export const CopyToClipboard = ({ text }) => {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    setCopied(false)
    const res = await copy(text)

    if (res.success) {
      setCopied(true)
    } else {
      console.log(res.value)
    }
  }

  return (
    <Button
      size="small"
      variant="outlined"
      aria-label="save"
      onClick={handleCopy}
      endIcon={copied && <Check style={{ color: green[500] }} />}
    >
      {copied ? 'Copied' : 'Copy'}
    </Button>
  )
}
