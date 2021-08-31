import React from 'react'
import { List, ListItem } from '@material-ui/core'

import { DocEntry } from './DocEntry'

export const ApiList = () => {
  return (
    <List>
      <ListItem>
        <DocEntry
          function={{
            name: 'connect_with_mnemonic',
            args: [{ name: 'mnemonic', type: 'str' }],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry function={{ name: 'get_balance', args: [] }} />
      </ListItem>
    </List>
  )
}
