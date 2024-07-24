import { FormControl, FormControlLabel, FormLabel, Radio, RadioGroup } from '@mui/material';
import { TNodeType } from '@nymproject/types';

export const NodeTypeSelector = ({
  disabled,
  nodeType,
  setNodeType,
}: {
  disabled: boolean;
  nodeType: TNodeType;
  setNodeType: (nodeType: TNodeType) => void;
}) => {
  const handleNodeTypeChange = (e: React.ChangeEvent<HTMLInputElement>) => setNodeType(e.target.value as TNodeType);

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
          value="mixnode"
          control={<Radio color="default" />}
          label="Mixnode"
          data-testid="mix-node"
          disabled={disabled}
        />
        <FormControlLabel
          value="gateway"
          control={<Radio color="default" />}
          data-testid="gate-way"
          label="Gateway"
          disabled={disabled}
        />
      </RadioGroup>
    </FormControl>
  );
};
