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
      <ListItem>
        <DocEntry
          function={{
            name: "owns_mixnode",
            args: [],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "owns_gateway",
            args: [],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "bond_mixnode",
            args: [
              { name: "mixnode", type: "object" },
              { name: "bond", type: "object" },
            ],
          }}
        />
      </ListItem>
    </List>
  )
}
