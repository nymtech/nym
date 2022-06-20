import React from 'react';
import { FormControl, FormControlLabel, FormLabel, Radio, RadioGroup } from '@mui/material';
import { EnumNodeType } from '@nymproject/types';

export const NodeTypeSelector = ({
  disabled,
  nodeType,
  setNodeType,
}: {
  disabled: boolean;
  nodeType: EnumNodeType;
  setNodeType: (nodeType: EnumNodeType) => void;
}) => {
  const handleNodeTypeChange = (e: React.ChangeEvent<HTMLInputElement>) => setNodeType(e.target.value as EnumNodeType);

  return (
    <FormControl component="fieldset">
      <FormLabel component="legend">Select node type</FormLabel>
      <RadioGroup
        aria-label="nodeType"
        name="nodeTypeRadio"
        value={nodeType}
        onChange={handleNodeTypeChange}
        style={{ display: 'block' }}
      >
        <FormControlLabel
          value={EnumNodeType.mixnode}
          control={<Radio color="default" />}
          label="Mixnode"
          data-testid="mix-node"
          disabled={disabled}
        />
        <FormControlLabel
          value={EnumNodeType.gateway}
          control={<Radio color="default" />}
          data-testid="gate-way"
          label="Gateway"
          disabled={disabled}
        />
      </RadioGroup>
    </FormControl>
  );
};
