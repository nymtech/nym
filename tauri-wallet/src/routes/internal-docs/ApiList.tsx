import React, { useState } from 'react'
import {
  Button,
  Checkbox,
  FormControlLabel,
  Grid,
  InputAdornment,
  List,
  ListItem,
  TextField,
  Theme,
} from '@material-ui/core'
import { useTheme } from '@material-ui/styles'
import { DocEntry } from './DocEntry'

export const ApiList = () => {
  const [advancedShown, setAdvancedShown] = React.useState(false)

  const theme: Theme = useTheme()

  return (
    <List>
      <ListItem><DocEntry function={{ name: 'connect_with_mnemonic', args: [{ name: 'mnemonic', type: 'str' }] }} /></ListItem>
      <ListItem><DocEntry function={{ name: 'test-2', args: [] }} /></ListItem>
    </List>

  )
}
