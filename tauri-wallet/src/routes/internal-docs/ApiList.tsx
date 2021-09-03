import React, { useState } from "react";
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
} from "@material-ui/core";
import { useTheme } from "@material-ui/styles";
import { DocEntry } from "./DocEntry";

export const ApiList = () => {
  const [advancedShown, setAdvancedShown] = React.useState(false);

  const theme: Theme = useTheme();

  return (
    <List>
      <ListItem>
        <DocEntry
          function={{
            name: "connect_with_mnemonic",
            args: [{ name: "mnemonic", type: "str" }],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry function={{ name: "get_balance", args: [] }} />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "minor_to_major",
            args: [{ name: "amount", type: "str" }],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "major_to_minor",
            args: [{ name: "amount", type: "str" }],
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
      <ListItem>
        <DocEntry
          function={{
            name: "unbond_mixnode",
            args: [],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "bond_gateway",
            args: [
              { name: "gateway", type: "object" },
              { name: "bond", type: "object" },
            ],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "unbond_gateway",
            args: [],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "delegate_to_mixnode",
            args: [
              { name: "identity", type: "str" },
              { name: "amount", type: "object" },
            ],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "undelegate_from_mixnode",
            args: [{ name: "identity", type: "str" }],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "delegate_to_gateway",
            args: [
              { name: "identity", type: "str" },
              { name: "amount", type: "object" },
            ],
          }}
        />
      </ListItem>
      <ListItem>
        <DocEntry
          function={{
            name: "undelegate_from_gateway",
            args: [{ name: "identity", type: "str" }],
          }}
        />
      </ListItem>
    </List>
  );
};
