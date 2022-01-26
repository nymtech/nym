import { MenuItem, useMediaQuery } from '@mui/material';
import * as React from 'react';
import Select from '@mui/material/Select';
import { SelectInputProps } from '@mui/material/Select/SelectInput';
import { useTheme } from '@mui/material/styles';
import { SxProps } from '@mui/system';
import { MixNodeStatus } from './Status';
import {
  MixnodeStatus,
  MixnodeStatusWithAll,
} from '../../typeDefs/explorer-api';

// TODO: replace with i18n
const ALL_NODES = 'All nodes';

interface MixNodeStatusDropdownProps {
  status?: MixnodeStatusWithAll;
  sx?: SxProps;
  onSelectionChanged?: (status?: MixnodeStatusWithAll) => void;
}

export const MixNodeStatusDropdown: React.FC<MixNodeStatusDropdownProps> = ({
  status,
  onSelectionChanged,
  sx,
}) => {
  const theme = useTheme();
  const matches = useMediaQuery(theme.breakpoints.down('sm'));
  const [statusValue, setStatusValue] = React.useState<MixnodeStatusWithAll>(
    status || MixnodeStatusWithAll.all,
  );
  const onChange: SelectInputProps<MixnodeStatusWithAll>['onChange'] =
    React.useCallback(
      ({ target: { value } }) => {
        setStatusValue(value);
        if (onSelectionChanged) {
          onSelectionChanged(value);
        }
      },
      [onSelectionChanged],
    );

  return (
    <Select
      labelId="mixnodeStatusSelect_label"
      id="mixnodeStatusSelect"
      value={statusValue}
      onChange={onChange}
      renderValue={(value) => {
        switch (value) {
          case MixnodeStatusWithAll.active:
          case MixnodeStatusWithAll.standby:
          case MixnodeStatusWithAll.inactive:
            return <MixNodeStatus status={value as unknown as MixnodeStatus} />;
          default:
            return ALL_NODES;
        }
      }}
      sx={{
        width: matches ? 'auto' : 200,
        ...sx,
      }}
    >
      <MenuItem
        value={MixnodeStatus.active}
        data-testid="mixnodeStatusSelectOption_active"
      >
        <MixNodeStatus status={MixnodeStatus.active} />
      </MenuItem>
      <MenuItem
        value={MixnodeStatus.standby}
        data-testid="mixnodeStatusSelectOption_standby"
      >
        <MixNodeStatus status={MixnodeStatus.standby} />
      </MenuItem>
      <MenuItem
        value={MixnodeStatus.inactive}
        data-testid="mixnodeStatusSelectOption_inactive"
      >
        <MixNodeStatus status={MixnodeStatus.inactive} />
      </MenuItem>
      <MenuItem
        value={MixnodeStatusWithAll.all}
        data-testid="mixnodeStatusSelectOption_allNodes"
      >
        {ALL_NODES}
      </MenuItem>
    </Select>
  );
};

MixNodeStatusDropdown.defaultProps = {
  onSelectionChanged: undefined,
  status: undefined,
  sx: undefined,
};
