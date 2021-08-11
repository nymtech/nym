import React, { useState } from 'react'
import { CircularProgress, Fab, makeStyles, Tooltip } from '@material-ui/core'
import { Check, FileCopyOutlined } from '@material-ui/icons'
import { green, grey } from '@material-ui/core/colors'

const copy = (text: string): Promise<{ success: boolean; value: string }> => {
  return new Promise((resolve, reject) => {
    navigator.clipboard
      .writeText(text)
      .then(() =>
        setTimeout(() => resolve({ success: true, value: text }), 1000)
      )
      .catch(() => reject({ success: false, value: 'Failed to copy' }))
  })
}

const useStyles = makeStyles((theme) => ({
  root: {
    display: 'flex',
    alignItems: 'center',
  },
  wrapper: {
    margin: theme.spacing(1),
    position: 'relative',
  },
  fabInitial: {
    color: grey[600],
    backgroundColor: grey[100],
    '&:hover': {
      backgroundColor: grey[200],
    },
  },
  buttonSuccess: {
    backgroundColor: green[500],
    '&:hover': {
      backgroundColor: green[700],
    },
  },
  fabProgress: {
    color: grey[400],
    position: 'absolute',
    top: -5,
    left: -5,
    zIndex: 1,
  },
}))

export const CopyToClipboard = ({ text }) => {
  const [copied, setCopied] = useState(false)
  const [isCopying, setIsCopying] = useState(false)
  const classes = useStyles()

  const handleCopy = async () => {
    setIsCopying(true)
    setCopied(false)
    const res = await copy(text)

    if (res.success) {
      setCopied(true)
    } else {
      console.log('Unable to copy to clipboard')
    }
    setIsCopying(false)
  }

  return (
    <div className={classes.wrapper}>
      <Tooltip title={!copied ? 'Copy' : 'Copied'}>
        <Fab
          size="small"
          aria-label="save"
          color="primary"
          className={copied ? classes.buttonSuccess : classes.fabInitial}
          onClick={handleCopy}
        >
          {copied ? <Check /> : <FileCopyOutlined />}
        </Fab>
      </Tooltip>
      {isCopying && (
        <CircularProgress size={50} className={classes.fabProgress} />
      )}
    </div>
  )
}
