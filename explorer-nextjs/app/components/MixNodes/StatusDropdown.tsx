import * as React from 'react'
import { MenuItem } from '@mui/material'
import Select from '@mui/material/Select'
import { SelectChangeEvent } from '@mui/material/Select/SelectInput'
import { SxProps } from '@mui/system'
import {
  MixnodeStatus,
  MixnodeStatusWithAll,
} from '@/app/typeDefs/explorer-api'
import { useIsMobile } from '@/app/hooks/useIsMobile'
import { MixNodeStatus } from './Status'

// TODO: replace with i18n
const ALL_NODES = 'All nodes'

interface MixNodeStatusDropdownProps {
  status?: MixnodeStatusWithAll
  sx?: SxProps
  onSelectionChanged?: (status?: MixnodeStatusWithAll) => void
}

export const MixNodeStatusDropdown: FCWithChildren<
  MixNodeStatusDropdownProps
> = ({ status, onSelectionChanged, sx }) => {
  const isMobile = useIsMobile()
  const [statusValue, setStatusValue] = React.useState<MixnodeStatusWithAll>(
    status || MixnodeStatusWithAll.all
  )
  const onChange = React.useCallback(
    (event: SelectChangeEvent) => {
      setStatusValue(event.target.value as MixnodeStatusWithAll)
      if (onSelectionChanged) {
        onSelectionChanged(event.target.value as MixnodeStatusWithAll)
      }
    },
    [onSelectionChanged]
  )

  return (
    <Select
      labelId="mixnodeStatusSelect_label"
      id="mixnodeStatusSelect"
      value={statusValue}
      onChange={onChange}
      renderValue={(value) => {
        switch (value) {
          case 'active':
          case 'standby':
          case 'inactive':
            return <MixNodeStatus status={value as unknown as MixnodeStatus} />
          default:
            return ALL_NODES
        }
      }}
      sx={{
        width: isMobile ? '50%' : 200,
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
      <MenuItem value={'all'} data-testid="mixnodeStatusSelectOption_allNodes">
        {ALL_NODES}
      </MenuItem>
    </Select>
  )
}
