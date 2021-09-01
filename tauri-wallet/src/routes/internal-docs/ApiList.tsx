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
      <ListItem>
        <DocEntry
          function={{
            name: 'printable_balance_to_native',
            args: [{ name: 'amount', type: 'str' }],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: 'native_to_printable',
            args: [{ name: 'nativeValue', type: 'str' }],
          }}
        />
      </ListItem>
    </List>
  )
}
